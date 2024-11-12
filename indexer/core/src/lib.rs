use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::sync::{Arc, OnceLock};
use tokio::runtime::{Builder, Runtime};
use tokio::task;
// Initialize the runtime once using OnceLock
//static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Context {
    db: DatabaseConnection,
}

impl Context {
    pub async fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        Migrator::up(&self.db, None).await
    }

    async fn connect_db() -> Result<DatabaseConnection, sea_orm::DbErr> {
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

#[derive(Debug, Clone)]
pub struct App {
    context: Context,
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

        let db = runtime.block_on(async { Context::connect_db().await })?;

        Ok(App {
            context: Context { db },
            runtime: runtime.clone(),
        })
    }

    pub fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        self.runtime
            .block_on(async { self.context.migrate_db().await })
    }

    pub fn db_txn(&self) -> Result<DatabaseTransaction, anyhow::Error> {
        //let Some(runtime) = RUNTIME.get() else {
        //    anyhow::bail!("Runtime not initialized");
        //};

        let db_transaction = self
            .runtime
            .block_on(async { self.context.db.begin().await })?;

        Ok(db_transaction)
    }

    pub fn index_block(&self) {}

    pub fn index_transaction(&self) {}

    pub fn save_db_txn(&self, txn: DatabaseTransaction) -> Result<(), sea_orm::DbErr> {
        //let Some(runtime) = RUNTIME.get() else {
        //    anyhow::bail!("Runtime not initialized");
        //};
        //
        self.runtime.block_on(async { txn.commit().await })?;

        Ok(())
    }

    /// This will be used to ensure the tokio has no more tasks to run, when we gracefully stop
    /// Grug.
    pub fn wait_for_all_tasks(&self) {
        self.runtime.block_on(async {});
    }
}

impl App {
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
    use assertor::*;
    use sea_orm::ActiveModelTrait;
    use sea_orm::ConnectionTrait;
    use sea_orm::EntityTrait;
    use sea_orm::Set;

    /// This is when used from Grug, which isn't async. In such case `App` has its own Tokio
    /// runtime and we need to inject async functions
    #[test]
    fn should_migrate_db_and_create_block() {
        let app = app();
        app.runtime.block_on(async {
            check_empty_and_create_block(&app.context.db).await;
        });
    }

    /// This is when used from the httpd API, which is async. We dont need to use `App` runtime.
    #[tokio::test]
    async fn async_should_migrate_db_and_create_block() {
        let db = Context::connect_db().await.expect("Can't get DB");
        Migrator::up(&db, None).await.expect("Can't run migration");
        check_empty_and_create_block(&db).await;
    }

    #[test]
    fn should_use_db_transaction() {
        let app = app();
        let txn = app.db_txn().expect("Can't get db txn");
        app.runtime
            .block_on(async {
                check_empty_and_create_block(&txn).await;
                txn.commit().await?;
                Ok::<(), sea_orm::DbErr>(())
            })
            .expect("Can't commit txn");
    }

    #[test]
    fn should_use_db_transaction_in_multiple_steps() {
        let app = app();
        let db = app.db_txn().expect("Can't get db txn");

        // create the block
        app.runtime.block_on(async {
            let new_block = indexer_entity::blocks::ActiveModel {
                id: Set(Default::default()),
                block_height: Set(10),
                created_at: Set(Default::default()),
            };
            new_block.insert(&db).await.expect("Can't save block");
        });

        // commit the transaction
        app.runtime.block_on(async {
            db.commit().await.expect("Can't commit txn");
        });

        // ensure block was saved
        app.runtime
            .block_on(async {
                let block = indexer_entity::blocks::Entity::find()
                    .one(&app.context.db)
                    .await
                    .expect("Can't fetch blocks")
                    .expect("Non existing block");
                assert_that!(block.block_height).is_equal_to(10);
                Ok::<(), sea_orm::DbErr>(())
            })
            .expect("Can't commit txn");
    }

    fn app() -> App {
        let app = App::new().expect("Can't create app");
        app.migrate_db().expect("Can't migrate DB");
        app
    }

    async fn check_empty_and_create_block<C: ConnectionTrait>(db: &C) {
        let blocks = indexer_entity::blocks::Entity::find()
            .all(db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(blocks).is_empty();
        let new_block = indexer_entity::blocks::ActiveModel {
            id: Set(Default::default()),
            block_height: Set(10),
            created_at: Set(Default::default()),
        };
        new_block.insert(db).await.expect("Can't save block");
        let block = indexer_entity::blocks::Entity::find()
            .one(db)
            .await
            .expect("Can't fetch blocks")
            .expect("Non existing block");
        assert_that!(block.block_height).is_equal_to(10);
    }
}
