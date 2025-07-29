use {indexer_sql::pubsub::PubSub, sea_orm::DatabaseConnection, std::sync::Arc};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
}

impl From<indexer_sql::context::Context> for Context {
    fn from(ctx: indexer_sql::context::Context) -> Self {
        Self {
            db: ctx.db.clone(),
            pubsub: ctx.pubsub.clone(),
        }
    }
}
