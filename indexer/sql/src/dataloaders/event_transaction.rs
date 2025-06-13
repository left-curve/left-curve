use {
    crate::entity,
    async_graphql::{dataloader::*, *},
    sea_orm::{DatabaseConnection, entity::prelude::*},
    std::{collections::HashMap, sync::Arc},
};

pub struct EventTransactionDataLoader {
    pub db: DatabaseConnection,
}

impl Loader<entity::events::Model> for EventTransactionDataLoader {
    type Error = Arc<sea_orm::DbErr>;
    type Value = entity::transactions::Model;

    // This allows to do a single SQL query to fetch all transactions related to a list of events.
    async fn load(
        &self,
        keys: &[entity::events::Model],
    ) -> Result<HashMap<entity::events::Model, Self::Value>, Self::Error> {
        let transaction_ids = keys.iter().map(|m| m.transaction_id).collect::<Vec<_>>();
        let events_by_transaction_id = keys
            .iter()
            .filter_map(|m| m.transaction_id.map(|id| (id, m.clone())))
            .collect::<HashMap<_, _>>();

        let transactions_by_transaction_ids: HashMap<uuid::Uuid, Self::Value> =
            entity::transactions::Entity::find()
                .filter(entity::transactions::Column::Id.is_in(transaction_ids))
                .all(&self.db)
                .await?
                .into_iter()
                .map(|t| (t.id, t))
                .collect();

        Ok(events_by_transaction_id
            .into_iter()
            .filter_map(|(id, event)| {
                transactions_by_transaction_ids
                    .get(&id)
                    .map(|transaction| (event, transaction.clone()))
            })
            .collect())
    }
}
