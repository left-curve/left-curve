use {
    async_graphql::{ComplexObject, SimpleObject},
    chrono::{DateTime, TimeZone, Utc},
    indexer_sql::entity,
    serde::Deserialize,
};

#[derive(Clone, Debug, SimpleObject, Deserialize, Default, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
#[graphql(complex)]
#[serde(default)]
pub struct Message {
    pub block_height: u64,
    pub created_at: DateTime<Utc>,
    pub order_idx: i32,
    pub method_name: String,
    pub contract_addr: Option<String>,
    pub sender_addr: String,
}

impl From<entity::messages::Model> for Message {
    fn from(item: entity::messages::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            order_idx: item.order_idx,
            method_name: item.method_name.clone(),
            contract_addr: item.contract_addr.clone(),
            sender_addr: item.sender_addr.clone(),
        }
    }
}

#[ComplexObject]
impl Message {}
