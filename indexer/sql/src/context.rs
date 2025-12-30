use {
    crate::{
        event_cache::EventCacheWriter,
        pubsub::{MemoryPubSub, PostgresPubSub, PubSub},
    },
    indexer_sql_migration::{Migrator, MigratorTrait},
    sea_orm::{
        ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
        sqlx::{self},
    },
    std::sync::Arc,
};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub event_cache: EventCacheWriter,
    // If set during tests, dropping the last Context will attempt to drop the temporary DB
    #[allow(dead_code)]
    pub test_cleanup: Option<Arc<TestDbCleanupGuard>>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("db", &self.db)
            .field("pubsub", &"<PubSub trait object>")
            .field("event_cache", &"<EventCacheWriter>")
            .field(
                "test_cleanup",
                &self
                    .test_cleanup
                    .as_ref()
                    .map(|_| "<TestDbCleanupGuard>")
                    .unwrap_or("<None>"),
            )
            .finish()
    }
}

impl Context {
    /// Create a new context with the same database connection but a separate pubsub instance
    /// This allows independent indexers to share the DB connection pool but have their own pubsub
    pub async fn with_separate_pubsub(&self) -> Result<Self, sea_orm::DbErr> {
        let new_pubsub: Arc<dyn PubSub<u64> + Send + Sync> = match &self.db {
            DatabaseConnection::SqlxPostgresPoolConnection(_) => {
                let pool: &sqlx::PgPool = self.db.get_postgres_connection_pool();
                Arc::new(
                    PostgresPubSub::new(pool.clone(), "blocks")
                        .await
                        .map_err(|e| {
                            sea_orm::DbErr::Custom(format!("Failed to create PostgresPubSub: {e}"))
                        })?,
                )
            },
            _ => {
                // For non-Postgres databases, use in-memory pubsub
                Arc::new(MemoryPubSub::new(100))
            },
        };

        Ok(Context {
            db: self.db.clone(),
            pubsub: new_pubsub,
            event_cache: self.event_cache.clone(),
            test_cleanup: self.test_cleanup.clone(),
        })
    }

    pub async fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        Migrator::up(&self.db, None).await
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

#[derive(Debug)]
pub struct TestDbCleanupGuard {
    pub server_prefix: String, // without trailing '/'
    pub db_name: String,
}

impl Drop for TestDbCleanupGuard {
    fn drop(&mut self) {
        // Best-effort; never panic from Drop
        let server_prefix = self.server_prefix.clone();
        let db_name = self.db_name.clone();

        let _ = std::panic::catch_unwind(|| {
            // If we're inside a Tokio runtime, spawn a detached task and return.
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let parent = format!("{}/postgres", server_prefix);
                    if let Ok(conn) = Database::connect(parent).await {
                        let drop_sql = format!("DROP DATABASE \"{}\" WITH (FORCE)", db_name);
                        if conn.execute_unprepared(&drop_sql).await.is_err() {
                            let _ = conn
                                .execute_unprepared(&format!("DROP DATABASE \"{}\"", db_name))
                                .await;
                        }
                    }
                });
                return;
            }

            // Otherwise, create a lightweight current-thread runtime and block
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                rt.block_on(async move {
                    let parent = format!("{}/postgres", server_prefix);
                    if let Ok(conn) = Database::connect(parent).await {
                        let drop_sql = format!("DROP DATABASE \"{}\" WITH (FORCE)", db_name);
                        if conn.execute_unprepared(&drop_sql).await.is_err() {
                            let _ = conn
                                .execute_unprepared(&format!("DROP DATABASE \"{}\"", db_name))
                                .await;
                        }
                    }
                });
            }
        });
    }
}
