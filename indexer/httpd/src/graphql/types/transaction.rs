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
pub struct Transaction {
    pub block_height: u64,
    pub created_at: DateTime<Utc>,
    // TODO: This should be an enum
    pub transaction_type: i16,
    pub transaction_idx: u32,
    pub sender: String,
    pub hash: String,
    pub has_succeeded: bool,
    pub error_message: Option<String>,
    pub gas_wanted: i64,
    pub gas_used: i64,
}

impl From<entity::transactions::Model> for Transaction {
    fn from(item: entity::transactions::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            transaction_type: item.transaction_type,
            transaction_idx: item.transaction_idx as u32,
            sender: item.sender.clone(),
            hash: item.hash.clone(),
            has_succeeded: item.has_succeeded,
            error_message: item.error_message.clone(),
            gas_wanted: item.gas_wanted,
            gas_used: item.gas_used,
        }
    }
}

#[ComplexObject]
impl Transaction {}
