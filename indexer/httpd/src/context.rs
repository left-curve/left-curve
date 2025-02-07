use {
    crate::error::Error,
    indexer_sql::pubsub::{MemoryPubSub, PostgresPubSub, PubSub},
    sea_orm::{ConnectOptions, Database, DatabaseConnection},
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
}

impl From<indexer_sql::Context> for Context {
    fn from(ctx: indexer_sql::Context) -> Self {
        Self {
            db: ctx.db,
            pubsub: ctx.pubsub,
        }
    }
}

impl Context {
    /// Create a new context with a database connection, will use postgres pubsub if the database is postgres
    pub async fn new(database_url: Option<String>) -> Result<Self, Error> {
        if let Some(database_url) = database_url {
            let db = Self::connect_db_with_url(&database_url).await?;
            if let DatabaseConnection::SqlxPostgresPoolConnection(_) = db {
                let pool = db.get_postgres_connection_pool();
                return Ok(Self {
                    db: db.clone(),
                    pubsub: Arc::new(PostgresPubSub::new(pool.clone())),
                });
            }
        }

        Ok(Self {
            db: Self::connect_db().await?,
            pubsub: Arc::new(MemoryPubSub::new(100)),
        })
    }

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
