use {
    crate::entities::CandleInterval,
    chrono::{DateTime, Utc},
    clickhouse::Row,
    grug::{Udec128_6, Udec128_24},
    serde::{Deserialize, Serialize},
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
    quote_denom: String,
    base_denom: String,
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
