use {
    crate::entity,
    grug_math::Inner,
    grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome},
    indexer_core::{Context, IndexerTrait},
    sea_orm::{
        prelude::*, sqlx::types::chrono::TimeZone, ActiveModelTrait, DatabaseTransaction, Set,
        TransactionTrait,
    },
    std::sync::{Arc, Mutex},
    tokio::runtime::{Builder, Runtime},
};

#[derive(Debug, Clone)]
pub struct Indexer {
    pub context: Context,
    pub runtime: Arc<Runtime>,
    /// Stores the current block height and the associated database transaction.
    db_txn: Arc<Mutex<Option<(u64, DatabaseTransaction)>>>,
}

impl Indexer {
    pub fn new() -> Result<Indexer, anyhow::Error> {
        let runtime = Arc::new(Builder::new_multi_thread()
                //.worker_threads(4)  // Adjust as needed
                .enable_all()
                .build()
                .unwrap());

        let db = runtime.block_on(async { Context::connect_db().await })?;

        Ok(Indexer {
            context: Context { db },
            runtime: runtime.clone(),
            db_txn: Arc::new(Mutex::new(None)),
        })
    }

    pub fn new_with_database(database_url: &str) -> Result<Indexer, anyhow::Error> {
        let runtime = Arc::new(Builder::new_multi_thread()
                //.worker_threads(4)  // Adjust as needed
                .enable_all()
                .build()
                .unwrap());

        let db = runtime.block_on(async { Context::connect_db_with_url(database_url).await })?;

        Ok(Indexer {
            context: Context { db },
            runtime: runtime.clone(),
            db_txn: Arc::new(Mutex::new(None)),
        })
    }
}

impl IndexerTrait for Indexer {
    fn start(&self) -> Result<(), anyhow::Error> {
        self.runtime
            .block_on(async { self.context.migrate_db().await })?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        //// This is making sure the current transaction is being committed before we shutdown the
        //// process
        let mut db_txn = self.db_txn.lock().unwrap();
        let Some((_, txn)) = db_txn.take() else {
            return Ok(());
        };

        self.runtime.block_on(async {
            txn.commit().await.expect("Can't commit txn");
        });

        *db_txn = None;
        Ok(())
    }

    fn pre_indexing(&self, block_height: u64) -> Result<(), anyhow::Error> {
        let db_transaction = self
            .runtime
            .block_on(async { self.context.db.begin().await })?;

        self.db_txn
            .lock()
            .unwrap()
            .replace((block_height, db_transaction));

        Ok(())
    }

    fn index_block(
        &self,
        block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error> {
        let db_txn = self.db_txn.lock().unwrap();
        let Some((block_height, txn)) = db_txn.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };

        assert_eq!(*block_height, block.height);

        self.runtime.block_on(async {
            let epoch_millis = block.timestamp.into_millis();
            let seconds = (epoch_millis / 1_000) as i64;
            let nanoseconds = ((epoch_millis % 1_000) * 1_000_000) as u32;

            let naive_datetime = sea_orm::sqlx::types::chrono::Utc
                .timestamp_opt(seconds, nanoseconds)
                .single()
                .unwrap_or_default()
                .naive_utc();

            let new_block = entity::blocks::ActiveModel {
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
        let db_txn = self.db_txn.lock().unwrap();
        let Some((block_height, txn)) = db_txn.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };

        assert_eq!(*block_height, block.height);

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
            let new_transaction = entity::transactions::ActiveModel {
                id: Set(transaction_id),
                has_succeeded: Set(tx_outcome.result.is_ok()),
                error_message: Set(tx_outcome.clone().result.err()),
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

                let new_message = entity::messages::ActiveModel {
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
                let new_event = entity::events::ActiveModel {
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
    fn post_indexing(&self, block_height: u64) -> Result<(), anyhow::Error> {
        let mut db_txn = self.db_txn.lock().unwrap();
        let Some((txn_block_height, txn)) = db_txn.take() else {
            anyhow::bail!("No transaction to commit");
        };

        assert_eq!(block_height, txn_block_height);

        self.runtime.block_on(async { txn.commit().await })?;

        Ok(())
    }
}

impl Indexer {
    /// This will be used to ensure the tokio has no more tasks to run, when we gracefully stop
    /// Grug. We don't inject async tasks yet (they all block) but could.
    pub fn wait_for_all_tasks(&self) {
        self.runtime.block_on(async {});
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        self.shutdown().expect("Can't shutdown Indexer");
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        assertor::*,
        migration::{Migrator, MigratorTrait},
        sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set},
    };

    /// This is when used from Grug, which isn't async. In such case `App` has its own Tokio
    /// runtime and we need to inject async functions
    #[test]
    fn should_migrate_db_and_create_block() -> anyhow::Result<()> {
        let app = app();
        app.pre_indexing(1).expect("Can't create db txn");
        let db = app.db_txn.lock().unwrap();
        let Some((_, db)) = db.as_ref() else {
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
    #[allow(clippy::await_holding_lock)]
    fn should_use_db_transaction() -> Result<(), anyhow::Error> {
        let app = app();
        app.pre_indexing(1)?;
        app.runtime
            .block_on(async {
                let db_guard = app.db_txn.lock().unwrap();
                let Some((_, db)) = db_guard.as_ref() else {
                    panic!("No transaction to commit");
                };
                check_empty_and_create_block(db).await;
                Ok::<(), anyhow::Error>(())
            })
            .expect("Can't commit txn");
        app.post_indexing(1)?;
        Ok(())
    }

    #[test]
    fn should_use_db_transaction_in_multiple_steps() -> Result<(), anyhow::Error> {
        let app = app();
        app.pre_indexing(1)?;
        let mut guard_db = app.db_txn.lock().unwrap();
        let Some((_, db)) = guard_db.as_ref() else {
            anyhow::bail!("No transaction to commit");
        };

        // create the block
        app.runtime.block_on(async {
            let new_block = entity::blocks::ActiveModel {
                id: Set(Default::default()),
                block_height: Set(10),
                created_at: Set(Default::default()),
                hash: Set(Default::default()),
            };
            new_block.insert(db).await.expect("Can't save block");
        });

        let Some((_, db)) = guard_db.take() else {
            anyhow::bail!("No transaction to commit");
        };

        // commit the transaction
        app.runtime.block_on(async {
            db.commit().await.expect("Can't commit txn");
        });

        // ensure block was saved
        app.runtime
            .block_on(async {
                let block = entity::blocks::Entity::find()
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

    fn app() -> Indexer {
        let app = Indexer::new().expect("Can't create indexer");
        app.start().expect("Can't start Indexer");
        app
    }

    async fn check_empty_and_create_block<C: ConnectionTrait>(db: &C) {
        let blocks = entity::blocks::Entity::find()
            .all(db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(blocks).is_empty();
        let new_block = entity::blocks::ActiveModel {
            id: Set(Default::default()),
            block_height: Set(10),
            created_at: Set(Default::default()),
            hash: Set(Default::default()),
        };
        new_block.insert(db).await.expect("Can't save block");
        let block = entity::blocks::Entity::find()
            .one(db)
            .await
            .expect("Can't fetch blocks")
            .expect("Non existing block");
        assert_that!(block.block_height).is_equal_to(10);
    }
}
