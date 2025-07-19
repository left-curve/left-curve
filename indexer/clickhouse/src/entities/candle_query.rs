use {
    crate::entities::candle::{Candle, CandleInterval},
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::Timestamp,
    itertools::Itertools,
    serde::Deserialize,
};

const MAX_ITEMS: usize = 100;

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
    limit: usize,
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
            limit: MAX_ITEMS,
        }
    }

    pub fn with_earlier_than(mut self, earlier_than: DateTime<Utc>) -> Self {
        self.earlier_than = Some(earlier_than);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = std::cmp::min(limit, MAX_ITEMS);
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

        let has_next_page = rows.len() > self.limit - 1;
        if has_next_page {
            rows.pop();
        }

        Ok(CandleResult {
            candles: rows,
            has_next_page,
            has_previous_page,
        })
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
        let query = format!(
            r#"SELECT
                maxMerge(block_height) as block_height
               FROM {}
               WHERE quote_denom = ? AND base_denom = ?"#,
            self.table_name()
        );

        #[derive(Row, Deserialize)]
        struct BlockHeight {
            block_height: u64,
        }

        let result: BlockHeight = clickhouse_client
            .query(&query)
            .bind(self.quote_denom.clone())
            .bind(self.base_denom.clone())
            .fetch_one()
            .await?;

        Ok(result.block_height)
    }

    pub fn table_name(&self) -> &str {
        match self.interval {
            CandleInterval::OneSecond => "pair_prices_1s",
            CandleInterval::OneMinute => "pair_prices_1m",
            CandleInterval::FiveMinutes => "pair_prices_5m",
            CandleInterval::FifteenMinutes => "pair_prices_15m",
            CandleInterval::OneHour => "pair_prices_1h",
            CandleInterval::FourHours => "pair_prices_4h",
            CandleInterval::OneDay => "pair_prices_1d",
            CandleInterval::OneWeek => "pair_prices_1w",
        }
    }

    pub fn materialized_table_name(&self) -> &str {
        match self.interval {
            CandleInterval::OneSecond => "pair_prices_1s_mv",
            CandleInterval::OneMinute => "pair_prices_1m_mv",
            CandleInterval::FiveMinutes => "pair_prices_5m_mv",
            CandleInterval::FifteenMinutes => "pair_prices_15m_mv",
            CandleInterval::OneHour => "pair_prices_1h_mv",
            CandleInterval::FourHours => "pair_prices_4h_mv",
            CandleInterval::OneDay => "pair_prices_1d_mv",
            CandleInterval::OneWeek => "pair_prices_1w_mv",
        }
    }

    fn query_string(&self) -> (String, Vec<String>, bool) {
        let interval_str = self.interval.to_string();
        let mut has_previous_page = false;

        let mut query = format!(
            r#"SELECT
                        quote_denom,
                        base_denom,
                        time_start,
                        argMinMerge(open) AS open,
                        maxMerge(high) AS high,
                        minMerge(low) AS low,
                        argMaxMerge(close) AS close,
                        sumMerge(volume_base) AS volume_base,
                        sumMerge(volume_quote) as volume_quote,
                        maxMerge(block_height) as block_height,
                        '{interval_str}' as interval
                       FROM {}
                       WHERE quote_denom = ? AND base_denom = ?"#,
            self.table_name()
        );

        let mut params: Vec<String> = vec![self.quote_denom.clone(), self.base_denom.clone()];

        if let Some(earlier_than) = self.earlier_than {
            query.push_str(" AND time_start <= ?");
            params.push(Timestamp::from(earlier_than.naive_utc()).to_rfc3339_string());
        }

        if let Some(later_than) = self.later_than {
            query.push_str(" AND time_start >= ?");
            params.push(Timestamp::from(later_than.naive_utc()).to_rfc3339_string());
        }

        if let Some(after) = self.after {
            query.push_str(" AND time_start < ?");
            params.push(Timestamp::from(after.naive_utc()).to_rfc3339_string());
            has_previous_page = true;
        }

        query.push_str(" GROUP BY quote_denom, base_denom, time_start");

        query.push_str(" ORDER BY time_start DESC");

        query.push_str(&format!(" LIMIT {}", self.limit + 1));

        (query, params, has_previous_page)
    }
}
