#[cfg(feature = "async-graphql")]
use {
    crate::entities::graphql_decimal::GraphqlBigDecimal,
    async_graphql::{ComplexObject, SimpleObject},
    bigdecimal::BigDecimal,
    bigdecimal::num_bigint::BigInt,
    grug::Inner,
    grug::Timestamp,
};
use {
    crate::entities::{CandleInterval, perps_pair_price::PerpsPairPrice},
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::{NumberConst, Udec128_6},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Row, Serialize, Deserialize, Eq, PartialEq, Hash, Clone)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
pub struct PerpsCandle {
    pub pair_id: String,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
    pub time_start: DateTime<Utc>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub open: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub high: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub low: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub close: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub volume: Udec128_6,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    #[serde(with = "crate::entities::pair_price::dec")]
    pub volume_usd: Udec128_6,
    pub min_block_height: u64,
    pub max_block_height: u64,
    pub interval: CandleInterval,
}

impl PerpsCandle {
    /// Creates a new candle from a perps pair price.
    pub fn new_with_pair_price(
        pair_price: &PerpsPairPrice,
        interval: CandleInterval,
        time_start: DateTime<Utc>,
        block_height: u64,
    ) -> Self {
        PerpsCandle {
            pair_id: pair_price.pair_id.clone(),
            time_start,
            open: pair_price.close,
            high: pair_price.high,
            low: pair_price.low,
            close: pair_price.close,
            volume: pair_price.volume,
            volume_usd: pair_price.volume_usd,
            interval,
            max_block_height: block_height,
            min_block_height: block_height,
        }
    }

    /// Creates a new candle from a previous candle.
    pub fn new_with_previous_candle(
        previous_candle: &PerpsCandle,
        interval: CandleInterval,
        time_start: DateTime<Utc>,
        block_height: u64,
    ) -> Self {
        PerpsCandle {
            pair_id: previous_candle.pair_id.clone(),
            time_start,
            open: previous_candle.close,
            high: previous_candle.close,
            low: previous_candle.close,
            close: previous_candle.close,
            volume: Udec128_6::ZERO,
            volume_usd: Udec128_6::ZERO,
            interval,
            max_block_height: block_height,
            min_block_height: block_height,
        }
    }

    pub fn set_high_low(&mut self, high: Udec128_6, low: Udec128_6) {
        self.high = self.high.max(high);
        self.low = self.low.min(low);
    }
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PerpsCandle {
    #[graphql(deprecation = "Use `maxBlockHeight` instead")]
    async fn block_height(&self) -> u64 {
        self.max_block_height
    }

    async fn open(&self) -> GraphqlBigDecimal {
        let inner_value = self.open.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized().into()
    }

    async fn high(&self) -> GraphqlBigDecimal {
        let inner_value = self.high.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized().into()
    }

    async fn low(&self) -> GraphqlBigDecimal {
        let inner_value = self.low.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized().into()
    }

    async fn close(&self) -> GraphqlBigDecimal {
        let inner_value = self.close.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized().into()
    }

    async fn volume(&self) -> GraphqlBigDecimal {
        let inner_value = self.volume.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized().into()
    }

    async fn volume_usd(&self) -> GraphqlBigDecimal {
        let inner_value = self.volume_usd.inner();
        let bigint = BigInt::from(*inner_value);
        BigDecimal::new(bigint, 6).normalized().into()
    }

    /// Return time_start in ISO 8601 format with time zone.
    async fn time_start(&self) -> String {
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
