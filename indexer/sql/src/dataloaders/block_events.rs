#[cfg(feature = "async-graphql")]
use async_graphql::{dataloader::*, *};
use {
    crate::entity,
    itertools::Itertools,
    sea_orm::{DatabaseConnection, Order, QueryOrder, entity::prelude::*},
    std::{collections::HashMap, sync::Arc},
};

pub struct BlockEventsDataLoader {
    pub db: DatabaseConnection,
}

#[cfg(feature = "async-graphql")]
impl Loader<entity::blocks::Model> for BlockEventsDataLoader {
    type Error = Arc<sea_orm::DbErr>;
    type Value = Vec<entity::events::Model>;

    // This allows to do a single SQL query to fetch all transactions related to a list of blocks.
    async fn load(
        &self,
        keys: &[entity::blocks::Model],
    ) -> Result<HashMap<entity::blocks::Model, Self::Value>, Self::Error> {
        let block_block_heights = keys.iter().map(|m| m.block_height).collect::<Vec<_>>();
        let blocks_by_height = keys
            .iter()
            .map(|m| (m.block_height, m.clone()))
            .collect::<HashMap<_, _>>();

        let events_by_block_heights: HashMap<i64, Vec<entity::events::Model>> =
            entity::events::Entity::find()
                .filter(entity::events::Column::BlockHeight.is_in(block_block_heights))
                .order_by(entity::events::Column::BlockHeight, Order::Asc)
                .order_by(entity::events::Column::EventIdx, Order::Asc)
                .all(&self.db)
                .await?
                .into_iter()
                .chunk_by(|t| t.block_height)
                .into_iter()
                .map(|(key, group)| (key, group.collect::<Self::Value>()))
                .collect();

        Ok(blocks_by_height
            .into_iter()
            .map(|(block_height, block)| {
                let events = events_by_block_heights
                    .get(&block_height)
                    .unwrap_or(&vec![])
                    .clone();
                (block, events)
            })
            .collect())
    }
}
