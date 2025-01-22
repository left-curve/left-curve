use {
    indexer_sql_migration::{Migrator, MigratorTrait},
    sea_orm::{ConnectOptions, Database, DatabaseConnection},
};

#[derive(Debug, Clone)]
pub struct Context {
    pub db: DatabaseConnection,
}

impl Context {
    pub async fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        Migrator::up(&self.db, None).await
    }

    pub async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
        let database_url = "sqlite::memory:";

        Self::connect_db_with_url(database_url).await
    }

    pub async fn connect_db_with_url(
        database_url: &str,
    ) -> Result<DatabaseConnection, sea_orm::DbErr> {
        let mut opt = ConnectOptions::new(database_url.to_owned());
        opt.max_connections(10);
        // .min_connections(5)
        //.connect_timeout(Duration::from_secs(settings.timeout))
        //.idle_timeout(Duration::from_secs(8))
        //.max_lifetime(Duration::from_secs(20))
        //.sqlx_logging(settings.logging);

        Database::connect(opt).await
    }
}
