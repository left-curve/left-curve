use {
    async_graphql::{ComplexObject, SimpleObject},
    chrono::{DateTime, TimeZone, Utc},
    dango_indexer_sql::entity,
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Transfer {
    pub block_height: u64,
    pub idx: i32,
    pub created_at: DateTime<Utc>,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub denom: String,
}

impl From<entity::transfers::Model> for Transfer {
    fn from(item: entity::transfers::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            idx: item.idx,
            created_at: Utc.from_utc_datetime(&item.created_at),
            from_address: item.from_address,
            to_address: item.to_address,
            amount: item.amount,
            denom: item.denom,
        }
    }
}

#[ComplexObject]
impl Transfer {}
