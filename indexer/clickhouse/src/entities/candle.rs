use {
    crate::{Dec, Int},
    chrono::{DateTime, Duration, Utc},
    clickhouse::Row,
    grug::{Udec128, Uint128},
    serde::{Deserialize, Deserializer, Serialize, Serializer, de},
    strum::EnumIter,
    strum_macros::{Display, EnumString},
};
#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, Enum, SimpleObject},
    bigdecimal::BigDecimal,
    grug::Timestamp,
};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Display, EnumString, EnumIter)]
#[cfg_attr(feature = "async-graphql", derive(Enum))]
#[cfg_attr(feature = "async-graphql", graphql(name = "CandleInterval"))]
pub enum CandleInterval {
    #[strum(serialize = "1s")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "ONE_SECOND"))]
    OneSecond,
    #[strum(serialize = "1m")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "ONE_MINUTE"))]
    OneMinute,
    #[strum(serialize = "5m")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "FIVE_MINUTES"))]
    FiveMinutes,
    #[strum(serialize = "15m")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "FIFTEEN_MINUTES"))]
    FifteenMinutes,
    #[strum(serialize = "1h")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "ONE_HOUR"))]
    OneHour,
    #[strum(serialize = "4h")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "FOUR_HOURS"))]
    FourHours,
    #[strum(serialize = "1d")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "ONE_DAY"))]
    OneDay,
    #[strum(serialize = "1w")]
    #[cfg_attr(feature = "async-graphql", graphql(name = "ONE_WEEK"))]
    OneWeek,
}

impl CandleInterval {
    pub fn duration(&self) -> Duration {
        match self {
            CandleInterval::OneSecond => Duration::seconds(1),
            CandleInterval::OneMinute => Duration::seconds(60),
            CandleInterval::FiveMinutes => Duration::seconds(300),
            CandleInterval::FifteenMinutes => Duration::seconds(900),
            CandleInterval::OneHour => Duration::seconds(3600),
            CandleInterval::FourHours => Duration::seconds(14400),
            CandleInterval::OneDay => Duration::seconds(86400),
            CandleInterval::OneWeek => Duration::seconds(604800),
        }
    }
}

impl Serialize for CandleInterval {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for CandleInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Self>()
            .map_err(|e| de::Error::custom(e.to_string()))
    }
}

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
    pub open: Dec<Udec128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub high: Dec<Udec128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub low: Dec<Udec128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub close: Dec<Udec128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub volume_base: Int<Uint128>,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub volume_quote: Int<Uint128>,
    pub interval: CandleInterval,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl Candle {
    /// Return time_start in ISO 8601 format with time zone.
    async fn time_start(&self) -> String {
        // TODO: check why microseconds are not included
        Timestamp::from(self.time_start.naive_utc()).to_rfc3339_string()
    }

    /// Return time_start as a unix timestamp
    async fn time_start_unix(&self) -> i64 {
        self.time_start.timestamp_millis()
    }

    async fn open(&self) -> BigDecimal {
        BigDecimal::from(self.open.clone()).normalized()
    }

    async fn high(&self) -> BigDecimal {
        BigDecimal::from(self.high.clone()).normalized()
    }

    async fn low(&self) -> BigDecimal {
        BigDecimal::from(self.low.clone()).normalized()
    }

    async fn close(&self) -> BigDecimal {
        BigDecimal::from(self.close.clone()).normalized()
    }

    async fn volume_base(&self) -> String {
        self.volume_base.to_string()
    }

    async fn volume_quote(&self) -> String {
        self.volume_quote.to_string()
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
