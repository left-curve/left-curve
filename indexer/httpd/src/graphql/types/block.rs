use {
    super::{event::Event, transaction::Transaction},
    crate::graphql::dataloader::{
        block_events::BlockEventsDataLoader, block_transactions::BlockTransactionsDataLoader,
    },
    async_graphql::{ComplexObject, Context, Result, SimpleObject, dataloader::DataLoader},
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
    pub transactions_count: i32,
}

impl From<entity::blocks::Model> for Block {
    fn from(item: entity::blocks::Model) -> Self {
        Self {
            block_height: item.block_height as u64,
            created_at: Utc.from_utc_datetime(&item.created_at),
            hash: item.hash,
            app_hash: item.app_hash,
            transactions_count: item.transactions_count,
        }
    }
}

#[ComplexObject]
impl Block {
    /// Transactions order isn't guaranteed, check `transactionIdx`
    async fn transactions(&self, ctx: &Context<'_>) -> Result<Vec<Transaction>> {
        let loader = ctx.data_unchecked::<DataLoader<BlockTransactionsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }

    async fn flatten_events(&self, ctx: &Context<'_>) -> Result<Vec<Event>> {
        let loader = ctx.data_unchecked::<DataLoader<BlockEventsDataLoader>>();
        Ok(loader.load_one(self.clone()).await?.unwrap_or_default())
    }
}

#[derive(SimpleObject)]
pub struct BlockInfo {
    pub block_height: u64,
    pub timestamp: DateTime<Utc>,
    pub hash: String,
}

impl From<grug_types::BlockInfo> for BlockInfo {
    fn from(item: grug_types::BlockInfo) -> Self {
        Self {
            block_height: item.height,
            timestamp: item.timestamp.to_utc_date_time(),
            hash: item.hash.to_string(),
        }
    }
}
