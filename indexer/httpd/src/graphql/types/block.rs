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

#[derive(SimpleObject)]
pub struct BlockInfo {
    pub block_height: u64,
    pub timestamp: DateTime<Utc>,
    pub hash: String,
}

impl From<grug_types::BlockInfo> for BlockInfo {
    fn from(item: grug_types::BlockInfo) -> Self {
        let epoch_millis = item.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let timestamp = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default();
        Self {
            block_height: item.height,
            timestamp,
            hash: item.hash.to_string(),
        }
    }
}
