use {crate::pubsub::PubSub, sea_orm::DatabaseConnection, std::sync::Arc};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
}
