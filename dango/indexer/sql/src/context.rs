use {
    indexer_sql::pubsub::{MemoryPubSub, PubSub},
    sea_orm::DatabaseConnection,
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
}

impl From<indexer_sql::context::Context> for Context {
    fn from(ctx: indexer_sql::context::Context) -> Self {
        Self {
            db: ctx.db.clone(),
            pubsub: Arc::new(MemoryPubSub::new(100)),
        }
    }
}
