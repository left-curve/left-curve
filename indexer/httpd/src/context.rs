use {
    crate::traits::ConsensusClient,
    grug_httpd::{context::Context as BaseContext, traits::QueryApp},
    indexer_sql::{indexer_path::IndexerPath, pubsub::PubSub},
    sea_orm::{ConnectOptions, Database, DatabaseConnection},
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub base: BaseContext,
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub consensus_client: Arc<dyn ConsensusClient + Send + Sync>,
    pub indexer_path: IndexerPath,
}

impl Context {
    pub fn new(
        ctx: indexer_sql::Context,
        grug_app: Arc<dyn QueryApp + Send + Sync>,
        consensus_client: Arc<dyn ConsensusClient + Send + Sync>,
        indexer_path: IndexerPath,
    ) -> Self {
        Self {
            base: BaseContext::new(grug_app),
            db: ctx.db,
            pubsub: ctx.pubsub,
            consensus_client,
            indexer_path,
        }
    }

    pub fn grug_app(&self) -> &Arc<dyn QueryApp + Send + Sync> {
        &self.base.grug_app
    }
}

impl Context {
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
