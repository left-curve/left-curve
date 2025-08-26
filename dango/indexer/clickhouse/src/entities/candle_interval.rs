#[cfg(feature = "async-graphql")]
use async_graphql::Enum;
use {
    chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday},
    serde::{Deserializer, Serialize, Serializer, de},
    strum::EnumIter,
    strum_macros::{Display, EnumString},
};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Display, EnumString, EnumIter)]
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
    // Helper to align a timestamp to the start of its candle interval (ClickHouse style)
    pub fn interval_start(&self, ts: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            CandleInterval::OneWeek => {
                // Week starts on Sunday for Clickhouse and I must align the timestamp accordingly
                let days_since_sunday = match ts.weekday() {
                    Weekday::Sun => 0,
                    Weekday::Mon => 1,
                    Weekday::Tue => 2,
                    Weekday::Wed => 3,
                    Weekday::Thu => 4,
                    Weekday::Fri => 5,
                    Weekday::Sat => 6,
                };

                let start_of_day = ts.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let monday = start_of_day - Duration::days(days_since_sunday);
                Utc.from_utc_datetime(&monday)
            },
            _ => {
                let interval_secs = self.duration().num_seconds();
                assert!(interval_secs > 0, "Interval duration must be > 0");

                let ts_secs = ts.timestamp();
                let aligned = ts_secs - (ts_secs % interval_secs);
                DateTime::from_timestamp(aligned, 0).expect("valid aligned timestamp")
            },
        }
    }

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

    pub fn table_name(&self) -> &str {
        match self {
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
        match self {
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
