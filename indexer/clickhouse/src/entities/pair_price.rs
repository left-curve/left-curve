use {
    chrono::NaiveDateTime,
    clickhouse::Row,
    grug::{Denom, Udec128},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Row, Serialize, Deserialize)]
pub struct PairPrice {
    pub denoms: (Denom, Denom),
    pub clearing_price: Udec128,
    pub timestamp: NaiveDateTime,
    pub block_height: u64,
}
