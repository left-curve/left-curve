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
pub struct Block {
    pub block_height: u64,
    pub created_at: DateTime<Utc>,
    pub hash: String,
    pub app_hash: String,
}

impl From<entity::blocks::Model> for Block {
    fn from(item: entity::blocks::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            hash: item.hash,
            app_hash: item.app_hash,
        }
    }
}

#[ComplexObject]
impl Block {}
