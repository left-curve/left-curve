use {
    crate::entities::{clearing_price::ClearingPrice, volume::Volume},
    chrono::{DateTime, Utc},
    clickhouse::Row,
    serde::{Deserialize, Deserializer, Serialize, Serializer, de},
    strum_macros::{Display, EnumString},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Display, EnumString)]
pub enum CandleInterval {
    #[strum(serialize = "1m")]
    OneMinute,
    #[strum(serialize = "5m")]
    FiveMinutes,
    #[strum(serialize = "15m")]
    FifteenMinutes,
    #[strum(serialize = "1h")]
    OneHour,
    #[strum(serialize = "4h")]
    FourHours,
    #[strum(serialize = "1d")]
    OneDay,
    #[strum(serialize = "1w")]
    OneWeek,
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
pub struct Candle {
    quote_denom: String,
    base_denom: String,
    #[serde(with = "clickhouse::serde::chrono::datetime64::micros")]
    time_start: DateTime<Utc>,
    open: ClearingPrice,
    high: ClearingPrice,
    low: ClearingPrice,
    close: ClearingPrice,
    volume_base: Volume,
    volume_quote: Volume,
    interval: CandleInterval,
}
