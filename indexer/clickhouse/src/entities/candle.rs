use {
    crate::entities::pair_price::{ClearingPrice, Volume},
    chrono::{DateTime, Utc},
    clickhouse::Row,
    serde::{Deserialize, Serialize},
};

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
}
