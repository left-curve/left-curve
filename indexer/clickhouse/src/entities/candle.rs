use {
    crate::{
        entities::CandleInterval,
        error::{IndexerError, Result},
    },
    chrono::{DateTime, Utc},
    clickhouse::Row,
    dango_types::dex::PairId,
    grug::{Denom, Udec128_6, Udec128_24},
    serde::{Deserialize, Serialize},
    std::str::FromStr,
};
#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::BigDecimal,
    bigdecimal::num_bigint::BigInt,
    grug::Inner,
    grug::Timestamp,
};

#[derive(Debug, Row, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
pub struct Candle {
    pub quote_denom: String,
    pub base_denom: String,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
    pub time_start: DateTime<Utc>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "super::pair_price::dec")]
    pub open: Udec128_24,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "super::pair_price::dec")]
    pub high: Udec128_24,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "super::pair_price::dec")]
    pub low: Udec128_24,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "super::pair_price::dec")]
    pub close: Udec128_24,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "super::pair_price::dec")]
    pub volume_base: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "super::pair_price::dec")]
    pub volume_quote: Udec128_6,
    pub interval: CandleInterval,
    pub block_height: u64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Candle {
    async fn open(&self) -> BigDecimal {
        let inner_value = self.open.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 24).normalized()
    }

    async fn high(&self) -> BigDecimal {
        let inner_value = self.high.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 24).normalized()
    }

    async fn low(&self) -> BigDecimal {
        let inner_value = self.low.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 24).normalized()
    }

    async fn close(&self) -> BigDecimal {
        let inner_value = self.close.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 24).normalized()
    }

    async fn volume_base(&self) -> BigDecimal {
        let inner_value = self.volume_base.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    async fn volume_quote(&self) -> BigDecimal {
        let inner_value = self.volume_quote.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized()
    }

    /// Return time_start in ISO 8601 format with time zone.
    async fn time_start(&self) -> String {
        // TODO: check why microseconds are not included
        Timestamp::from(self.time_start.naive_utc()).to_rfc3339_string()
    }

    /// Return time_start as a unix timestamp
    async fn time_start_unix(&self) -> i64 {
        self.time_start.timestamp_millis()
    }

    /// Return time_end in ISO 8601 format with time zone.
    async fn time_end(&self) -> String {
        Timestamp::from(self.time_start.naive_utc() + self.interval.duration()).to_rfc3339_string()
    }

    /// Return time_end as a unix timestamp
    async fn time_end_unix(&self) -> i64 {
        self.time_start.timestamp_millis() + self.interval.duration().num_milliseconds()
    }
}

impl Candle {
    /// Returns all existing pairs for a given interval and an optional block height.
    pub async fn existing_pairs(
        interval: CandleInterval,
        clickhouse_client: &clickhouse::Client,
        block_height: Option<u64>,
    ) -> Result<Vec<PairId>> {
        let mut query = format!(
            "SELECT DISTINCT base_denom, quote_denom FROM {}",
            interval.table_name()
        );

        let mut params: Vec<String> = Vec::new();

        if let Some(block_height) = block_height {
            query.push_str(" WHERE finalizeAggregation(block_height) = ?");
            params.push(block_height.to_string());
        }

        let mut clickhouse_query = clickhouse_client.query(&query);
        for param in params {
            clickhouse_query = clickhouse_query.bind(param);
        }

        let all_pairs: Vec<PairId> = clickhouse_query
            .fetch_all::<(String, String)>()
            .await?
            .into_iter()
            .map(|(base_denom, quote_denom)| {
                Ok::<PairId, IndexerError>(PairId {
                    base_denom: Denom::from_str(&base_denom)?,
                    quote_denom: Denom::from_str(&quote_denom)?,
                })
            })
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        Ok(all_pairs)
    }

    /// Returns pairs that are missing for a given interval and block height.
    pub async fn get_missing_pairs(
        interval: CandleInterval,
        clickhouse_client: &clickhouse::Client,
        block_height: u64,
    ) -> Result<Vec<PairId>> {
        let all_pairs = Self::existing_pairs(interval, clickhouse_client, None).await?;

        let existing_pairs =
            Self::existing_pairs(interval, clickhouse_client, Some(block_height)).await?;

        Ok(all_pairs
            .into_iter()
            .filter(|pair| !existing_pairs.contains(pair))
            .collect())
    }

    /// Returns the last block height for a given interval and pair.
    pub async fn last_block_height(
        interval: CandleInterval,
        clickhouse_client: &clickhouse::Client,
        pair: PairId,
    ) -> Result<Option<u64>> {
        let query = format!(
            "SELECT max(finalizeAggregation(block_height)) FROM {} WHERE (quote_denom = ? AND base_denom = ?)",
            interval.table_name()
        );

        let last_block_height: Option<u64> = clickhouse_client
            .query(&query)
            .bind(pair.quote_denom)
            .bind(pair.base_denom)
            .fetch_optional()
            .await?;

        Ok(last_block_height)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {super::*, grug::Timestamp};

    #[test]
    fn test_time_start_with_microseconds() {
        let time_start: DateTime<Utc> = "1971-01-01T00:00:00.500Z".parse().unwrap();

        let time_start = Timestamp::from(time_start);
        assert_eq!(time_start.to_rfc3339_string(), "1971-01-01T00:00:00.500Z");
    }
}
