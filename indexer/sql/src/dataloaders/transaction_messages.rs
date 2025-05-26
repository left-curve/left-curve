#[cfg(feature = "async-graphql")]
use async_graphql::{dataloader::*, *};
use {
    crate::entity,
    itertools::Itertools,
    sea_orm::{DatabaseConnection, Order, QueryOrder, entity::prelude::*},
    std::{collections::HashMap, sync::Arc},
};

pub struct TransactionMessagesDataLoader {
    pub db: DatabaseConnection,
}

#[cfg(feature = "async-graphql")]
impl Loader<entity::transactions::Model> for TransactionMessagesDataLoader {
    type Error = Arc<sea_orm::DbErr>;
    type Value = Vec<entity::messages::Model>;

    // This allows to do a single SQL query to fetch all messages related to a list of transactions.
    async fn load(
        &self,
        keys: &[entity::transactions::Model],
    ) -> Result<HashMap<entity::transactions::Model, Self::Value>, Self::Error> {
        let transactions_ids = keys.iter().map(|m| m.id).collect::<Vec<_>>();
        let transactions_by_id = keys
            .iter()
            .map(|m| (m.id, m.clone()))
            .collect::<HashMap<_, _>>();

        let messages_by_transaction_ids: HashMap<uuid::Uuid, Self::Value> =
            entity::messages::Entity::find()
                // NOTE: this filtering could raise issue if `transaction_ids` is thousands of entries long
                //       as it would generate a SQL query with thousands of `OR` conditions
                .filter(entity::messages::Column::TransactionId.is_in(transactions_ids))
                .order_by(entity::messages::Column::OrderIdx, Order::Asc)
                .all(&self.db)
                .await?
                .into_iter()
                .into_group_map_by(|msg| msg.transaction_id);

        Ok(transactions_by_id
            .into_iter()
            .map(|(id, transaction)| {
                let messages = messages_by_transaction_ids
                    .get(&id)
                    .unwrap_or(&vec![])
                    .clone();
                (transaction, messages)
            })
            .collect())
    }
}
