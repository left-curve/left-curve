use {
    crate::entity,
    async_graphql::{dataloader::*, *},
    itertools::Itertools,
    sea_orm::{DatabaseConnection, Order, QueryOrder, entity::prelude::*},
    std::{collections::HashMap, sync::Arc},
};

pub struct TransactionEventsDataLoader {
    pub db: DatabaseConnection,
}

impl Loader<entity::transactions::Model> for TransactionEventsDataLoader {
    type Error = Arc<sea_orm::DbErr>;
    type Value = Vec<entity::events::Model>;

    // This allows to do a single SQL query to fetch all transactions related to a list of blocks.
    async fn load(
        &self,
        keys: &[entity::transactions::Model],
    ) -> Result<HashMap<entity::transactions::Model, Self::Value>, Self::Error> {
        let transactions_ids = keys.iter().map(|m| m.id).collect::<Vec<_>>();
        let transactions_by_id = keys
            .iter()
            .map(|m| (m.id, m.clone()))
            .collect::<HashMap<_, _>>();

        let events_by_transaction_ids: HashMap<uuid::Uuid, Self::Value> =
            entity::events::Entity::find()
                // NOTE: this filtering could raise issue if `transaction_ids` is thousands of entries long
                //       as it would generate a SQL query with thousands of `OR` conditions
                .filter(entity::events::Column::TransactionId.is_in(transactions_ids))
                // safeguard because we use `.transaction_id.expect("transaction_id is null")` below
                .filter(entity::events::Column::TransactionId.is_not_null())
                .order_by(entity::events::Column::EventIdx, Order::Asc)
                .all(&self.db)
                .await?
                .into_iter()
                .chunk_by(|t| t.transaction_id.expect("transaction_id is null"))
                .into_iter()
                .map(|(key, group)| {
                    (
                        key,
                        group.into_iter().collect::<Self::Value>(),
                    )
                })
                .collect();

        Ok(transactions_by_id
            .into_iter()
            .map(|(id, transaction)| {
                let events = events_by_transaction_ids
                    .get(&id)
                    .unwrap_or(&vec![])
                    .clone();
                (transaction, events)
            })
            .collect())
    }
}
