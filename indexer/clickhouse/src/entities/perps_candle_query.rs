#[cfg(feature = "tracing")]
use itertools::Itertools;
use {
    crate::entities::{CandleInterval, perps_candle::PerpsCandle},
    chrono::{DateTime, Utc},
};

pub const MAX_ITEMS: usize = 650;

#[derive(Debug, Clone)]
pub struct PerpsCandleResult {
    pub candles: Vec<PerpsCandle>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub struct PerpsCandleQueryBuilder {
    interval: CandleInterval,
    pair_id: String,
    earlier_than: Option<DateTime<Utc>>,
    later_than: Option<DateTime<Utc>>,
    after: Option<DateTime<Utc>>,
    limit: Option<usize>,
}

impl PerpsCandleQueryBuilder {
    pub fn new(interval: CandleInterval, pair_id: String) -> Self {
        Self {
            interval,
            pair_id,
            earlier_than: None,
            later_than: None,
            after: None,
            limit: Some(MAX_ITEMS),
        }
    }

    pub fn with_earlier_than(mut self, earlier_than: DateTime<Utc>) -> Self {
        self.earlier_than = Some(earlier_than);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(std::cmp::min(limit, MAX_ITEMS));
        self
    }

    pub fn without_limit(mut self) -> Self {
        self.limit = None;
        self
    }

    pub fn with_later_than(mut self, later_than: DateTime<Utc>) -> Self {
        self.later_than = Some(later_than);
        self
    }

    pub fn with_after(mut self, after: DateTime<Utc>) -> Self {
        self.after = Some(after);
        self
    }

    pub async fn fetch_all(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<PerpsCandleResult, crate::error::IndexerError> {
        let (query, params, has_previous_page) = self.query_string();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            params = params.iter().map(|p| p.to_string()).join(", "),
            "Fetching perps candles: {query}"
        );

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        let mut rows: Vec<PerpsCandle> = cursor_query.fetch_all().await?;

        let has_next_page = rows.len() > self.limit.unwrap_or_default();
        if has_next_page {
            rows.pop();
        }

        Ok(PerpsCandleResult {
            candles: rows,
            has_next_page,
            has_previous_page,
        })
    }

    pub async fn fetch_one(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<Option<PerpsCandle>, crate::error::IndexerError> {
        let (query, params, _) = self.query_string();

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        Ok(cursor_query.fetch_optional().await?)
    }

    fn query_string(&self) -> (String, Vec<String>, bool) {
        let mut has_previous_page = false;

        let mut query = r#"
              SELECT
                pair_id,
                time_start,
                open,
                high,
                low,
                close,
                volume,
                volume_usd,
                min_block_height,
                max_block_height,
                interval
              FROM perps_candles FINAL
              WHERE pair_id = ? AND interval = ?
            "#
        .to_string();

        let mut params: Vec<String> = vec![self.pair_id.clone(), self.interval.to_string()];

        if let Some(earlier_than) = self.earlier_than {
            query.push_str(" AND time_start <= toDateTime64(?, 6)");
            params.push(earlier_than.timestamp_micros().to_string());
        }

        if let Some(later_than) = self.later_than {
            query.push_str(" AND time_start >= toDateTime64(?, 6)");
            params.push(later_than.timestamp_micros().to_string());
        }

        if let Some(after) = self.after {
            query.push_str(" AND time_start < toDateTime64(?, 6)");
            params.push(after.timestamp_micros().to_string());
            has_previous_page = true;
        }

        query.push_str(" ORDER BY time_start DESC");
        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit + 1));
        }

        (query, params, has_previous_page)
    }
}
