use {
    crate::traits::{ConsensusClient, QueryApp},
    indexer_sql::{
        EventCacheReader, entity::perps_trade::PerpsTrade, pubsub::PubSub,
        write::perps_trades::PerpsTradeCache,
    },
    sea_orm::{ConnectOptions, Database, DatabaseConnection},
    std::sync::Arc,
    tokio::sync::RwLock,
};

/// Chain-only context — what `cfg.indexer.enabled = false` mode runs against.
/// Holds just the chain query app. `FullContext` embeds one of these as its
/// `base` field so `GrugQuery` resolvers work in either schema.
#[derive(Clone)]
pub struct MinimalContext {
    pub grug_app: Arc<dyn QueryApp + Send + Sync>,
}

impl MinimalContext {
    pub fn new(grug_app: Arc<dyn QueryApp + Send + Sync>) -> Self {
        Self { grug_app }
    }
}

#[derive(Clone)]
pub struct FullContext {
    pub sql_context: indexer_sql::Context,
    pub indexer_cache_context: indexer_cache::Context,
    pub clickhouse_context: indexer_clickhouse::context::Context,
    pub base: MinimalContext,
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub perps_trade_pubsub: Arc<dyn PubSub<PerpsTrade> + Send + Sync>,
    pub perps_trade_cache: Arc<RwLock<PerpsTradeCache>>,
    pub consensus_client: Arc<dyn ConsensusClient + Send + Sync>,
    pub event_cache: EventCacheReader,
    pub static_files_path: Option<String>,
}

impl FullContext {
    pub fn new(
        indexer_cache_context: indexer_cache::Context,
        ctx: indexer_sql::Context,
        clickhouse_context: indexer_clickhouse::context::Context,
        grug_app: Arc<dyn QueryApp + Send + Sync>,
        consensus_client: Arc<dyn ConsensusClient + Send + Sync>,
        static_files_path: Option<String>,
    ) -> Self {
        Self {
            indexer_cache_context,
            clickhouse_context,
            db: ctx.db.clone(),
            pubsub: ctx.pubsub.clone(),
            perps_trade_pubsub: ctx.perps_trade_pubsub.clone(),
            perps_trade_cache: ctx.perps_trade_cache.clone(),
            event_cache: ctx.event_cache.as_reader(),
            sql_context: ctx,
            base: MinimalContext::new(grug_app),
            consensus_client,
            static_files_path,
        }
    }

    pub fn grug_app(&self) -> &Arc<dyn QueryApp + Send + Sync> {
        &self.base.grug_app
    }

    /// Preload the perps trade cache from existing DB data so that new
    /// GraphQL subscribers immediately receive recent trades.
    pub async fn start_perps_trade_cache(&self) -> Result<(), indexer_sql::error::IndexerError> {
        let mut cache = self.perps_trade_cache.write().await;
        cache.preload(&self.db).await
    }
}

impl FullContext {
    pub async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
        let database_url = "sqlite::memory:";

        Self::connect_db_with_url(database_url).await
    }

    pub async fn connect_db_with_url(
        database_url: &str,
    ) -> Result<DatabaseConnection, sea_orm::DbErr> {
        let mut opt = ConnectOptions::new(database_url.to_owned());

        // Default number of workers is the number of logical CPUs, which is what actix is using
        // TODO: add this as a configuration flag
        let num_workers = num_cpus::get();

        opt.max_connections(num_workers as u32)
        //.min_connections(5)
        //.connect_timeout(Duration::from_secs(settings.timeout))
        //.idle_timeout(Duration::from_secs(8))
        //.max_lifetime(Duration::from_secs(20))
        .sqlx_logging(false);

        match Database::connect(opt).await {
            Ok(db) => {
                #[cfg(feature = "tracing")]
                tracing::info!(database_url, "Connected to database");

                Ok(db)
            },
            Err(error) => {
                #[cfg(feature = "tracing")]
                tracing::error!(database_url, %error, "Failed to connect to database");

                Err(error)
            },
        }
    }
}
