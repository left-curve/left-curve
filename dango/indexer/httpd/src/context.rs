use {
    dango_indexer_sql::{
        entity::perps_trade::PerpsTrade, indexer::perps_trades::cache::PerpsTradeCache,
    },
    indexer_sql::pubsub::PubSub,
    sea_orm::DatabaseConnection,
    std::sync::Arc,
    tokio::sync::RwLock,
};

#[derive(Clone)]
pub struct Context {
    pub indexer_httpd_context: indexer_httpd::context::Context,
    pub indexer_clickhouse_context: dango_indexer_clickhouse::context::Context,
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub perps_trade_pubsub: Arc<dyn PubSub<PerpsTrade> + Send + Sync>,
    pub perps_trade_cache: Arc<RwLock<PerpsTradeCache>>,
    pub static_files_path: Option<String>,
}

impl Context {
    pub fn new(
        indexer_httpd_context: indexer_httpd::context::Context,
        indexer_clickhouse_context: dango_indexer_clickhouse::context::Context,
        ctx: dango_indexer_sql::context::Context,
        static_files_path: Option<String>,
    ) -> Self {
        Self {
            indexer_httpd_context,
            indexer_clickhouse_context,
            db: ctx.db.clone(),
            pubsub: ctx.pubsub.clone(),
            perps_trade_pubsub: ctx.perps_trade_pubsub.clone(),
            perps_trade_cache: ctx.perps_trade_cache.clone(),
            static_files_path,
        }
    }

    /// Preload the perps trade cache from existing DB data so that new
    /// GraphQL subscribers immediately receive recent trades.
    pub async fn start_perps_trade_cache(&self) -> Result<(), dango_indexer_sql::error::Error> {
        let mut cache = self.perps_trade_cache.write().await;
        cache.preload(&self.db).await
    }
}
