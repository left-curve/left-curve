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

pub trait AppTrait {
    fn db_txn(&self) -> Result<(), anyhow::Error>;
    fn index_block(
        &self,
        block: &BlockInfo,
        block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error>;
    fn index_transaction(
        &self,
        block: &BlockInfo,
        tx: &Tx,
        tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error>;
    fn save_db_txn(&self) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Clone)]
pub struct NoApp;

impl AppTrait for NoApp {
    fn db_txn(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn index_block(
        &self,
        _block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn index_transaction(
        &self,
        _block: &BlockInfo,
        _tx: &Tx,
        _tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn save_db_txn(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct App {
    pub context: Context,
    pub runtime: Arc<Runtime>,
    db_txn: Arc<Mutex<Option<DatabaseTransaction>>>,
}

impl AppTrait for App {
    fn db_txn(&self) -> Result<(), anyhow::Error> {
        let db_transaction = self
            .runtime
            .block_on(async { self.context.db.begin().await })?;

        self.db_txn.lock().unwrap().replace(db_transaction);

        Ok(())
    }

    fn index_block(
        &self,
        block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error> {
        let txn = self.db_txn.lock().unwrap();
        let Some(txn) = txn.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };

        self.runtime.block_on(async {
            let epoch_millis = block.timestamp.into_millis();
            let seconds = (epoch_millis / 1_000) as i64;
            let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

            let naive_datetime = sea_orm::sqlx::types::chrono::Utc
                .timestamp_opt(seconds, nanoseconds)
                .single()
                .unwrap_or_default()
                .naive_utc();

            // TODO: implement a From &BlockInfo -> indexer_entity::blocks::ActiveModel
            let new_block = indexer_entity::blocks::ActiveModel {
                id: Set(Uuid::new_v4()),
                block_height: Set(block.height.try_into().unwrap()),
                created_at: Set(naive_datetime),
                hash: Set(block.hash.to_string()),
            };
            new_block.insert(txn).await.expect("Can't save block");
        });

        Ok(())
    }

    fn index_transaction(
        &self,
        block: &BlockInfo,
        tx: &Tx,
        tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error> {
        let txn = self.db_txn.lock().unwrap();
        let Some(txn) = txn.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };

        let epoch_millis = block.timestamp.into_millis();
        let seconds = (epoch_millis / 1_000) as i64;
        let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

        let naive_datetime = sea_orm::sqlx::types::chrono::Utc
            .timestamp_opt(seconds, nanoseconds)
            .single()
            .unwrap_or_default()
            .naive_utc();

        self.runtime.block_on(async {
            let transaction_id = Uuid::new_v4();
            let sender = tx.sender.to_string();
            let new_transaction = indexer_entity::transactions::ActiveModel {
                id: Set(transaction_id),
                has_succeeded: Set(tx_outcome.result.is_ok()),
                error_message: Set(tx_outcome
                    .clone()
                    .result
                    .map_or_else(|err| Some(err), |_| None)),
                gas_wanted: Set(tx.gas_limit.try_into().unwrap()),
                gas_used: Set(tx_outcome.gas_used.try_into().unwrap()),
                created_at: Set(naive_datetime),
                block_height: Set(block.height.try_into().unwrap()),
                hash: Set("".to_string()),
                data: Set(tx.data.clone().into_inner()),
                sender: Set(sender),
                credential: Set(tx.credential.clone().into_inner()),
            };
            new_transaction
                .insert(txn)
                .await
                .expect("Can't save transaction");
            for message in tx.msgs.iter() {
                let serialized_message = serde_json::to_value(message).unwrap();
                let contract_addr = serialized_message
                    .get("contract")
                    .and_then(|c| c.as_str())
                    .map(|c| c.to_string());
                let method_name = serialized_message
                    .as_object()
                    .and_then(|obj| obj.keys().next().cloned())
                    .unwrap_or_default();

                let new_message = indexer_entity::messages::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    transaction_id: Set(transaction_id),
                    block_height: Set(block.height.try_into().unwrap()),
                    created_at: Set(naive_datetime),
                    method_name: Set(method_name),
                    data: Set(serialized_message),
                    addr: Set(contract_addr),
                };
                new_message.insert(txn).await.expect("Can't save message");
            }
            for event in tx_outcome.events.iter() {
                let serialized_attributes = serde_json::to_value(&event.attributes).unwrap();
                let new_event = indexer_entity::events::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    transaction_id: Set(transaction_id),
                    block_height: Set(block.height.try_into().unwrap()),
                    created_at: Set(naive_datetime),
                    r#type: Set(event.r#type.clone()),
                    attributes: Set(serialized_attributes),
                };
                new_event.insert(txn).await.expect("Can't save event");
            }
        });

        Ok(())
    }

    /// NOTE: when calling this, the DB transaction Mutex but be unlocked!
    fn save_db_txn(&self) -> Result<(), anyhow::Error> {
        let mut txn = self.db_txn.lock().unwrap();
        let Some(txn) = txn.take() else {
            anyhow::bail!("No transaction to commit");
        };

        self.runtime.block_on(async { txn.commit().await })?;

        Ok(())
    }
}

impl App {
    pub fn new() -> Result<Self, anyhow::Error> {
        let runtime = Arc::new(Builder::new_multi_thread()
                //.worker_threads(4)  // Adjust as needed
                .enable_all()
                .build()
                .unwrap());

        let db = runtime.block_on(async { Context::connect_db().await })?;

        Ok(App {
            context: Context { db },
            runtime: runtime.clone(),
            db_txn: Arc::new(Mutex::new(None)),
        })
    }

    pub fn migrate_db(&self) -> Result<(), sea_orm::DbErr> {
        self.runtime
            .block_on(async { self.context.migrate_db().await })
    }

    /// This will be used to ensure the tokio has no more tasks to run, when we gracefully stop
    /// Grug. We don't inject async tasks yet (they all block) but could.
    pub fn wait_for_all_tasks(&self) {
        self.runtime.block_on(async {});
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // If the DatabaseTransaction is left open (not committed) its `Drop` implementation
        // expects a Tokio context. We must call `rollback` manually on it within our Tokio
        // context.
        let mut guard_db = self.db_txn.lock().unwrap();
        let Some(db) = guard_db.take() else {
            return;
        };

        self.runtime.block_on(async {
            db.rollback().await.expect("Can't rollback txn");
        });

        *guard_db = None;
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
    fn should_migrate_db_and_create_block() -> anyhow::Result<()> {
        let app = app();
        app.db_txn().expect("Can't create db txn");
        let db = app.db_txn.lock().unwrap();
        let Some(db) = db.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };
        app.runtime.block_on(async {
            check_empty_and_create_block(db).await;
        });
        Ok(())
    }

    /// This is when used from the httpd API, which is async. We dont need to use `App` runtime.
    #[tokio::test]
    async fn async_should_migrate_db_and_create_block() {
        let db = Context::connect_db().await.expect("Can't get DB");
        Migrator::up(&db, None).await.expect("Can't run migration");
        check_empty_and_create_block(&db).await;
    }

    #[test]
    fn should_use_db_transaction() -> Result<(), anyhow::Error> {
        let app = app();
        app.db_txn()?;
        app.runtime
            .block_on(async {
                let db_guard = app.db_txn.lock().unwrap();
                let Some(db) = db_guard.as_ref() else {
                    panic!("No transaction to commit");
                };
                check_empty_and_create_block(db).await;
                Ok::<(), anyhow::Error>(())
            })
            .expect("Can't commit txn");
        app.save_db_txn()?;
        Ok(())
    }

    #[test]
    fn should_use_db_transaction_in_multiple_steps() -> Result<(), anyhow::Error> {
        let app = app();
        app.db_txn()?;
        let mut guard_db = app.db_txn.lock().unwrap();
        let Some(db) = guard_db.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };

        // create the block
        app.runtime.block_on(async {
            let new_block = indexer_entity::blocks::ActiveModel {
                id: Set(Default::default()),
                block_height: Set(10),
                created_at: Set(Default::default()),
                hash: Set(Default::default()),
            };
            new_block.insert(db).await.expect("Can't save block");
        });

        let Some(db) = guard_db.take() else {
            anyhow::bail!("No transaction to commit");
        };

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

        Ok(())
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
            hash: Set(Default::default()),
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
