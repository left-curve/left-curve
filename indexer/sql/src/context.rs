use {
    crate::pubsub::PubSub,
    indexer_sql_migration::{Migrator, MigratorTrait},
    sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection},
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
}

impl Context {
    pub async fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        Migrator::up(&self.db, None).await
    }

    pub async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
        let database_url = "sqlite::memory:";

        Self::connect_db_with_url(database_url, 10).await
    }

    pub async fn connect_db_with_url(
        database_url: &str,
        max_connections: u32,
    ) -> Result<DatabaseConnection, sea_orm::DbErr> {
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
                tracing::info!(database_url, "Connected to database");

                // NOTE: not doing all but this is what we should do based on Claude Code:
                // In-memory + single connection: Skip all pragmas
                // File-based + single connection: Only use synchronous=NORMAL
                // Any database + multiple connections: Use all 3 pragmas
                if database_url.contains("sqlite") && !database_url.contains(":memory:") {
                    #[cfg(feature = "tracing")]
                    tracing::info!("SQLite database detected, enabling optimizations");

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
                tracing::error!(database_url, %error, "Failed to connect to database");

                Err(error)
            },
        }
    }
}
