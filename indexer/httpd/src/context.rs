use {
    grug_app::QueryApp,
    indexer_sql::pubsub::PubSub,
    sea_orm::{ConnectOptions, Database, DatabaseConnection},
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
    pub grug_app: Arc<dyn QueryApp + Send + Sync>,
    pub tendermint_endpoint: String,
}

impl Context {
    pub fn new(
        ctx: indexer_sql::Context,
        grug_app: Arc<dyn QueryApp + Send + Sync>,
        tendermint_endpoint: String,
    ) -> Self {
        Self {
            db: ctx.db,
            pubsub: ctx.pubsub,
            grug_app,
            tendermint_endpoint,
        }
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
                tracing::info!("Connected to database: {}", database_url);

                Ok(db)
            },
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to connect to database {}: {:?}", database_url, e);

                Err(e)
            },
        }
    }
}
