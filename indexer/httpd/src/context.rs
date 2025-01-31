use {
    crate::{
        error::Error,
        pubsub::{MemoryPubSub, PubSub},
    },
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
            pubsub: Arc::new(MemoryPubSub::new(100)),
        }
    }
}

impl Context {
    pub fn new_with_database_connection(db: DatabaseConnection) -> Self {
        Self {
            db,
            pubsub: Arc::new(MemoryPubSub::new(100)),
        }
    }

    pub async fn new(database_url: Option<String>) -> Result<Self, Error> {
        let db = match database_url {
            Some(database_url) => Self::connect_db_with_url(&database_url).await?,
            None => Self::connect_db().await?,
        };

        Ok(Self {
            db,
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

        Database::connect(opt).await
    }
}
