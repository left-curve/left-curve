use {
    crate::entity,
    async_graphql::{dataloader::*, *},
    sea_orm::{DatabaseConnection, entity::prelude::*},
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    },
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
        let transaction_ids = keys
            .iter()
            .filter_map(|m| m.transaction_id)
            .collect::<HashSet<_>>();

        let transactions_by_transaction_ids: HashMap<uuid::Uuid, Self::Value> =
            entity::transactions::Entity::find()
                .filter(entity::transactions::Column::Id.is_in(transaction_ids))
                .all(&self.db)
                .await?
                .into_iter()
                .map(|t| (t.id, t))
                .collect();

        Ok(keys
            .iter()
            .filter_map(|key| {
                key.transaction_id
                    .map(|id| {
                        transactions_by_transaction_ids
                            .get(&id)
                            .map(|t| (key.clone(), t.clone()))
                    })
                    .flatten()
            })
            .collect())
    }
}
