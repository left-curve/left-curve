use {indexer_sql::pubsub::PubSub, sea_orm::DatabaseConnection, std::sync::Arc};

#[derive(Clone)]
pub struct Context {
    pub indexer_httpd_context: indexer_httpd::context::Context,
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
}

impl Context {
    pub fn new(
        indexer_httpd_context: indexer_httpd::context::Context,
        ctx: dango_indexer_sql::context::Context,
    ) -> Self {
        Self {
            indexer_httpd_context,
            db: ctx.db.clone(),
            pubsub: ctx.pubsub.clone(),
        }
    }
}
