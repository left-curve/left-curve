use clickhouse::query::RowCursor;
#[cfg(feature = "tracing")]
use itertools::Itertools;

use {
    crate::entities::{CandleInterval, candle::Candle},
    chrono::{DateTime, Utc},
    clickhouse::Row,
    serde::Deserialize,
};

pub const MAX_ITEMS: usize = 100;

#[derive(Debug, Clone)]
pub struct CandleResult {
    pub candles: Vec<Candle>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub struct CandleQueryBuilder {
    interval: CandleInterval,
    base_denom: String,
    quote_denom: String,
    earlier_than: Option<DateTime<Utc>>,
    later_than: Option<DateTime<Utc>>,
    after: Option<DateTime<Utc>>,
    limit: Option<usize>,
}

impl CandleQueryBuilder {
    pub fn new(interval: CandleInterval, base_denom: String, quote_denom: String) -> Self {
        Self {
            interval,
            base_denom,
            quote_denom,
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
    ) -> Result<CandleResult, crate::error::IndexerError> {
        let (query, params, has_previous_page) = self.query_string();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            params = params.iter().map(|p| p.to_string()).join(", "),
            "Fetching candles: {query}"
        );

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        let mut rows: Vec<Candle> = cursor_query.fetch_all().await?;

        for candle in rows.iter_mut() {
            candle.interval = self.interval;
        }

        let has_next_page = rows.len() > self.limit.unwrap_or_default();
        if has_next_page {
            rows.pop();
        }

        Ok(CandleResult {
            candles: rows,
            has_next_page,
            has_previous_page,
        })
    }

    pub fn fetch(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<RowCursor<Candle>, crate::error::IndexerError> {
        let (query, params, _) = self.query_string();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            params = params.iter().map(|p| p.to_string()).join(", "),
            "Fetching candles: {query}"
        );

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        Ok(cursor_query.fetch::<Candle>()?)
    }

    pub async fn fetch_one(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<Option<Candle>, crate::error::IndexerError> {
        let (query, params, _) = self.query_string();

        let mut cursor_query = clickhouse_client.query(&query);
        for param in params {
            cursor_query = cursor_query.bind(param);
        }

        Ok(cursor_query.fetch_optional().await?)
    }

    pub async fn get_max_block_height(
        &self,
        clickhouse_client: &clickhouse::Client,
    ) -> Result<u64, crate::error::IndexerError> {
        let query = r#"SELECT
                maxMerge(block_height) as block_height
               FROM candles
               WHERE quote_denom = ? AND base_denom = ?"#;

        #[derive(Row, Deserialize)]
        struct BlockHeight {
            block_height: u64,
        }

        let result: BlockHeight = clickhouse_client
            .query(query)
            .bind(self.quote_denom.clone())
            .bind(self.base_denom.clone())
            .fetch_one()
            .await?;

        Ok(result.block_height)
    }

    fn query_string(&self) -> (String, Vec<String>, bool) {
        let mut has_previous_page = false;

        let mut query = r#"
              SELECT
                quote_denom,
                base_denom,
                time_start,
                open,
                high,
                low,
                close,
                volume_base,
                volume_quote,
                block_height,
                interval,
              FROM candles FINAL
              WHERE quote_denom = ? AND base_denom = ? AND interval = ?
            "#
        .to_string();

        let mut params: Vec<String> = vec![
            self.quote_denom.clone(),
            self.base_denom.clone(),
            self.interval.to_string(),
        ];

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
