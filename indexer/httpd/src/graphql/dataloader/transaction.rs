use {
    crate::graphql::types::{block::Block, transaction::Transaction},
    async_graphql::{dataloader::*, *},
    indexer_sql::entity,
    itertools::Itertools,
    sea_orm::{entity::prelude::*, DatabaseConnection, Order, QueryOrder},
    std::{collections::HashMap, sync::Arc},
};

pub struct TransactionDataLoader {
    pub db: DatabaseConnection,
}

impl Loader<Block> for TransactionDataLoader {
    type Error = Arc<sea_orm::DbErr>;
    type Value = Vec<Transaction>;

    // This allows to do a single SQL query to fetch all transactions related to a list of blocks.
    async fn load(&self, keys: &[Block]) -> Result<HashMap<Block, Self::Value>, Self::Error> {
        let block_block_heights = keys.iter().map(|m| m.block_height).collect::<Vec<_>>();
        let blocks_by_height = keys
            .iter()
            .map(|m| (m.block_height, m.clone()))
            .collect::<HashMap<_, _>>();

        let transactions_by_block_heights: HashMap<u64, Vec<Transaction>> =
            entity::transactions::Entity::find()
                .filter(entity::transactions::Column::BlockHeight.is_in(block_block_heights))
                .order_by(entity::transactions::Column::BlockHeight, Order::Asc)
                .order_by(entity::transactions::Column::TransactionIdx, Order::Asc)
                .all(&self.db)
                .await?
                .into_iter()
                .map(|transaction| transaction.into())
                .collect::<Vec<Transaction>>()
                .into_iter()
                .chunk_by(|t| t.block_height)
                .into_iter()
                .map(|(key, group)| (key, group.collect::<Self::Value>()))
                .collect();

        Ok(blocks_by_height
            .into_iter()
            .map(|(block_height, block)| {
                let transactions = transactions_by_block_heights
                    .get(&block_height)
                    .unwrap_or(&vec![])
                    .clone();
                (block, transactions)
            })
            .collect())
    }
}
