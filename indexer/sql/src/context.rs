use {
    crate::{
        dango_migration, entity::perps_trade::PerpsTrade, event_cache::EventCacheWriter,
        grug_migration, pubsub::PubSub, write::perps_trades::cache::PerpsTradeCache,
    },
    sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection},
    sea_orm_migration::MigratorTrait,
    std::sync::Arc,
    tokio::sync::RwLock,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub perps_trade_pubsub: Arc<dyn PubSub<PerpsTrade> + Send + Sync>,
    pub event_cache: EventCacheWriter,
    pub perps_trade_cache: Arc<RwLock<PerpsTradeCache>>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("db", &self.db)
            .field("pubsub", &"<PubSub trait object>")
            .field("perps_trade_pubsub", &"<PubSub trait object>")
            .field("event_cache", &"<EventCacheWriter>")
            .field("perps_trade_cache", &"<PerpsTradeCache>")
            .finish()
    }
}

impl Context {
    /// Run both the grug-side and dango-side sea-orm migrators sequentially.
    /// Each migrator maintains its own physical migration table
    /// (`grug_seaql_migrations` and `dango_seaql_migrations`).
    pub async fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        grug_migration::Migrator::up(&self.db, None).await?;
        dango_migration::Migrator::up(&self.db, None).await?;
        Ok(())
    }

    /// Preload the in-memory perps trade cache from recent `OrderFilled` events
    /// in the `perps_events` table.
    pub async fn preload_perps_trade_cache(&self) -> Result<(), crate::error::IndexerError> {
        let mut cache = self.perps_trade_cache.write().await;
        cache.preload(&self.db).await
    }

    /// Close the database connection. Call this before dropping if you're
    /// in a context that might not have a Tokio runtime.
    /// Note: DatabaseConnection is internally an Arc, so clone is cheap.
    pub async fn close(&self) {
        if let Err(_e) = self.db.clone().close().await {
            #[cfg(feature = "tracing")]
            tracing::warn!(error = %_e, "Error closing database connection");
        }
    }

    pub async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
        let database_url = "sqlite::memory:";

        Self::connect_db_with_url(database_url, 10).await
    }

    pub async fn connect_db_with_url(
        database_url: &str,
        mut max_connections: u32,
    ) -> Result<DatabaseConnection, sea_orm::DbErr> {
        // SQLite in-memory databases do not support multiple writers and it will lead to deadlocks
        // and random errors if we try to use more than one connection.
        if database_url.contains("sqlite")
            && database_url.contains(":memory:")
            && max_connections > 1
        {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "SQLite in-memory doesn't support multiple writers; forcing to 1 connection to avoid deadlocks"
            );

            max_connections = 1;
        }

        let mut opt = ConnectOptions::new(database_url.to_owned());
        opt.max_connections(max_connections)
        // .min_connections(5)
        //.connect_timeout(Duration::from_secs(settings.timeout))
        //.idle_timeout(Duration::from_secs(8))
        //.max_lifetime(Duration::from_secs(20))
        .sqlx_logging(false);

        match Database::connect(opt).await {
            Ok(db) => {
                #[cfg(feature = "tracing")]
                tracing::info!(database_url, max_connections, "Connected to database");

                // NOTE: not doing all but this is what we should do based on Claude Code:
                // In-memory + single connection: Skip all pragmas
                // File-based + single connection: Only use synchronous=NORMAL
                // Any database + multiple connections: Use all 3 pragmas
                if database_url.contains("sqlite") && !database_url.contains(":memory:") {
                    #[cfg(feature = "tracing")]
                    tracing::info!("SQLite non-memory database detected, enabling optimizations");

                    db.execute_unprepared(
                        "PRAGMA journal_mode=WAL;
                         PRAGMA busy_timeout=5000;
                         PRAGMA synchronous=NORMAL;",
                    )
                    .await?;
                }

                Ok(db)
            },
            Err(error) => {
                #[cfg(feature = "tracing")]
                tracing::error!(database_url, max_connections, %error, "Failed to connect to database");

                Err(error)
            },
        }
    }
}
