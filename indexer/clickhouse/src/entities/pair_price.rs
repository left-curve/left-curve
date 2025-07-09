#[cfg(feature = "async-graphql")]
use {
    async_graphql::{ComplexObject, SimpleObject},
    grug_types::Timestamp,
};
use {
    chrono::NaiveDateTime,
    clickhouse::Row,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Row, Serialize, Deserialize)]
#[cfg_attr(feature = "async-graphql", derive(SimpleObject))]
#[cfg_attr(feature = "async-graphql", graphql(complex))]
#[cfg_attr(feature = "async-graphql", graphql(name = "PairPrice"))]
#[cfg_attr(feature = "async-graphql", serde(rename_all = "camelCase"))]
pub struct PairPrice {
    pub base_denom: String,
    pub quote_denom: String,
    pub clearing_price: String,
    #[cfg_attr(feature = "async-graphql", graphql(skip))]
    pub created_at: NaiveDateTime,
    pub block_height: u64,
}

#[cfg(feature = "async-graphql")]
#[ComplexObject]
impl PairPrice {
    /// Returns the block timestamp in ISO 8601 format with time zone.
    async fn created_at(&self) -> String {
        Timestamp::from(self.created_at).to_rfc3339_string()
    }
}
