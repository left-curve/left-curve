use {
    crate::{entity::perps_trade::PerpsTrade, indexer::perps_trades::cache::PerpsTradeCache},
    indexer_sql::pubsub::{MemoryPubSub, PubSub},
    sea_orm::DatabaseConnection,
    std::sync::Arc,
    tokio::sync::RwLock,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub perps_trade_pubsub: Arc<dyn PubSub<PerpsTrade> + Send + Sync>,
    pub perps_trade_cache: Arc<RwLock<PerpsTradeCache>>,
}

impl Context {
    pub async fn preload_perps_trade_cache(&self) -> Result<(), crate::error::Error> {
        let mut cache = self.perps_trade_cache.write().await;
        cache.preload(&self.db).await
    }
}

impl From<indexer_sql::context::Context> for Context {
    fn from(ctx: indexer_sql::context::Context) -> Self {
        Self {
            db: ctx.db.clone(),
            pubsub: Arc::new(MemoryPubSub::new(100)),
            perps_trade_pubsub: Arc::new(MemoryPubSub::new(100)),
            perps_trade_cache: Default::default(),
        }
    }
}
