use {async_graphql::SimpleObject, chrono::NaiveDateTime};

#[derive(SimpleObject)]
pub struct BlockInfo {
    pub block_height: u64,
    pub timestamp: NaiveDateTime,
    pub hash: String,
}

impl From<grug_types::BlockInfo> for BlockInfo {
    fn from(item: grug_types::BlockInfo) -> Self {
        Self {
            block_height: item.height,
            timestamp: item.timestamp.to_naive_date_time(),
            hash: item.hash.to_string(),
        }
    }
}

#[derive(SimpleObject)]
pub struct Status {
    pub block: BlockInfo,
    pub chain_id: String,
}
