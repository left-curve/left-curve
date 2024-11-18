use grug_math::Inner;
use grug_types::JsonSerExt;
use grug_types::{BlockInfo, BlockOutcome, Message, Tx, TxOutcome};
use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::*;
use sea_orm::sqlx::types::chrono::TimeZone;
use sea_orm::ActiveModelTrait;
use sea_orm::ConnectionTrait;
use sea_orm::EntityTrait;
use sea_orm::Set;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::sync::{Arc, Mutex};
use tokio::runtime::{Builder, Runtime};
use tokio::task;

#[derive(Debug, Clone)]
pub struct Context {
    pub db: DatabaseConnection,
}

impl Context {
    pub async fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        Migrator::up(&self.db, None).await
    }

    pub(crate) async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
        // TODO: Use the settings to connect to the database
        let database_url = "sqlite::memory:";

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
