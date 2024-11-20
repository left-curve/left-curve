use super::Context;
use super::IndexerTrait;
use crate::active_model::Models;
use grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome};
use sea_orm::ActiveModelTrait;
use sea_orm::EntityTrait;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::{Builder, Runtime};

/// Because I'm using `.spawn` in this implementation, I ran into lifetime issues where I need the
/// data to live as long as the spawned task.
///
/// I also have potential issues where the task spawned in `pre_indexing` to create a DB
/// transaction (in the sync implentation of this trait) could be theorically executed after the
/// task spawned in `index_block` and `index_transaction` meaning I'd have to check in these
/// functions if the transaction exists or not.
///
/// Decided to do different and prepare the data in memory to inject all data in a single Tokio
/// spawned task
#[derive(Debug, Clone)]
pub struct Indexer {
    pub context: Context,
    pub runtime: Arc<Runtime>,
    blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
}

/// Saves the block and its transactions in memory
#[derive(Debug)]
struct BlockToIndex {
    pub block_info: BlockInfo,
    pub txs: Vec<(Tx, TxOutcome)>,
}

impl BlockToIndex {
    /// Takes care of inserting the data in the database
    pub async fn save(self, db: &DatabaseTransaction) -> Result<(), sea_orm::DbErr> {
        let mut models = Models::build(&self.block_info);
        for tx in self.txs.iter() {
            models.push(&tx.0, &tx.1);
        }

        // TODO: if the process was to crash in the middle and restarted, we could try to
        // reinsert existing data. We should use `on_conflict()` to avoid this, return the
        // existing block and change `block_id` when/if we added foreign keys
        models.block.insert(db).await?;
        indexer_entity::transactions::Entity::insert_many(models.transactions)
            .exec(db)
            .await?;
        indexer_entity::messages::Entity::insert_many(models.messages)
            .exec(db)
            .await?;
        indexer_entity::events::Entity::insert_many(models.events)
            .exec(db)
            .await?;
        Ok(())
    }
}

impl Indexer {
    fn find_or_create<F, R>(&self, block: &BlockInfo, action: F) -> Result<R, anyhow::Error>
    where
        F: FnOnce(&mut BlockToIndex) -> Result<R, anyhow::Error>,
    {
        let mut blocks = self.blocks.lock().expect("Can't lock blocks");
        let block_to_index = blocks.entry(block.height).or_insert(BlockToIndex {
            block_info: block.clone(),
            txs: vec![],
        });
        action(block_to_index)
    }

    fn remove_or_fail(&self, block_height: &u64) -> Result<BlockToIndex, anyhow::Error> {
        let mut blocks = self.blocks.lock().expect("Can't lock blocks");
        let block_to_index = match blocks.remove_entry(block_height) {
            Some(block) => block,
            None => anyhow::bail!("Block {} not found", block_height),
        };
        Ok(block_to_index.1)
    }
}

impl IndexerTrait for Indexer {
    fn new() -> Result<Self, anyhow::Error> {
        let runtime = Arc::new(Builder::new_multi_thread()
                //.worker_threads(4)  // Adjust as needed
                .enable_all()
                .build()
                .unwrap());

        let db = runtime.block_on(async { Context::connect_db().await })?;

        Ok(Indexer {
            context: Context { db },
            runtime: runtime.clone(),
            blocks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn start(&self) -> Result<(), anyhow::Error> {
        self.runtime
            .block_on(async { self.context.migrate_db().await })?;
        Ok(())
    }

    fn shutdown(self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn index_block(
        &self,
        block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error> {
        self.find_or_create(block, |_block_to_index| Ok(()))
    }

    fn index_transaction(
        &self,
        block: &BlockInfo,
        tx: &Tx,
        tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error> {
        self.find_or_create(block, |block_to_index| {
            block_to_index.txs.push((tx.clone(), tx_outcome.clone()));

            Ok(())
        })
    }

    fn post_indexing(&self, block_height: u64) -> Result<(), anyhow::Error> {
        let context = self.context.clone();
        let block_to_index = self.remove_or_fail(&block_height)?;

        self.runtime.spawn(async move {
            let db = context.db.begin().await?;
            block_to_index.save(&db).await?;
            db.commit().await?;

            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        // If the DatabaseTransactions are left open (not committed) its `Drop` implementation
        // expects a Tokio context. We must call `rollback` manually on it within our Tokio
        // context.

        let mut blocks = self.blocks.lock().expect("Can't lock blocks");

        // We can block since this will only be called when program stops
        self.runtime.block_on(async {
            let db = self
                .context
                .db
                .begin()
                .await
                .expect("Can't get DB transaction");
            for (_block_height, block_to_index) in blocks.drain() {
                block_to_index
                    .save(&db)
                    .await
                    .expect("Can't save block_to_index");
            }
            db.commit().await.expect("Can't commit DB txn");
        });
    }
}
