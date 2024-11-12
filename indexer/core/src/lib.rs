use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Builder, Runtime};
use tokio::task;
// Initialize the runtime once using OnceLock
//static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct App {
    db: DatabaseConnection,
    runtime: Arc<Runtime>,
}

impl App {
    pub fn new() -> Result<Self, anyhow::Error> {
        //let runtime = RUNTIME.get_or_init(|| {
        //    Arc::new(Builder::new_multi_thread()
        //        //.worker_threads(4)  // Adjust as needed
        //        .enable_all()
        //        .build()
        //        .unwrap())
        //});

        let runtime = Arc::new(Builder::new_multi_thread()
                //.worker_threads(4)  // Adjust as needed
                .enable_all()
                .build()
                .unwrap());

        //let runtime = Builder::new_multi_thread().enable_all().build()?;

        let db = runtime.block_on(async { Self::connect_db().await })?;

        Ok(App {
            db,
            runtime: runtime.clone(),
        })
    }

    pub fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        self.runtime
            .block_on(async { self.migrate_db_async().await })
    }

    pub async fn migrate_db_async(&self) -> Result<(), sea_orm::DbErr> {
        Migrator::up(&self.db, None).await
    }

    pub fn precommit(&self) -> Result<DatabaseTransaction, anyhow::Error> {
        //let Some(runtime) = RUNTIME.get() else {
        //    anyhow::bail!("Runtime not initialized");
        //};

        let db_transaction = self.runtime.block_on(async { self.db.begin().await })?;

        Ok(db_transaction)
    }

    pub fn index_block(&self) {}

    pub fn index_transaction(&self) {}

    pub fn commit(&self, txn: DatabaseTransaction) -> Result<(), anyhow::Error> {
        //let Some(runtime) = RUNTIME.get() else {
        //    anyhow::bail!("Runtime not initialized");
        //};
        //
        self.runtime.block_on(async { txn.commit().await })?;

        Ok(())
    }
}

impl App {
    async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
        // TODO: Use the settings to connect to the database
        let database_url = "sqlite::memory:";

        // TODO: migrate the database

        let mut opt = ConnectOptions::new(database_url.to_owned());
        opt.max_connections(10);
        // .min_connections(5)
        //.connect_timeout(Duration::from_secs(settings.timeout))
        //.idle_timeout(Duration::from_secs(8))
        //.max_lifetime(Duration::from_secs(20))
        //.sqlx_logging(settings.logging);
        Database::connect(opt).await
    }

    #[allow(dead_code)]
    // Function to run an async task without blocking
    fn run_async_task<F>(&self, task: F)
    where
        F: FnOnce() -> tokio::task::JoinHandle<()> + Send + 'static,
    {
        self.runtime.spawn(task());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::entity::prelude::DateTime;
    use sea_orm::ActiveModelTrait;
    use sea_orm::EntityTrait;
    use sea_orm::Set;
    use uuid::Uuid;

    #[test]
    fn should_migrate_db() {
        let app = App::new().unwrap();
        app.migrate_db().expect("Can't migrate DB");
        app.runtime.block_on(async {
            let blocks = indexer_entity::blocks::Entity::find()
                .all(&app.db)
                .await
                .expect("Can't fetch blocks");
            assert_eq!(blocks.len(), 0);
            let new_block = indexer_entity::blocks::ActiveModel {
                id: Set(Uuid::new_v4()),
                //created_at: Set(DateTime::new()),
                block_height: Set(0),
                ..Default::default()
            };
            new_block.insert(&app.db).await.expect("Can't save block");
            let blocks = indexer_entity::blocks::Entity::find()
                .all(&app.db)
                .await
                .expect("C'ant fetch blocks");
            assert_eq!(blocks.len(), 1);
        });
    }

    #[tokio::test]
    async fn should_migrate_db_and_have_no_block() {
        let app = App::new().unwrap();
        app.migrate_db().expect("Can't migrate DB");
    }
}
