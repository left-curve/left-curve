use {
    super::{Context, IndexerTrait},
    crate::active_model::Models,
    grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome},
    sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, TransactionTrait},
    std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::{Builder, Runtime},
};

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
/// NOTE: Do not make this `Clone`
#[derive(Debug)]
pub struct Indexer {
    pub context: Context,
    pub runtime: Arc<Runtime>,
    blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
    indexing: bool,
}

/// Saves the block and its transactions in memory
#[derive(Debug, Clone)]
struct BlockToIndex {
    pub block_info: BlockInfo,
    pub txs: Vec<(Tx, TxOutcome)>,
}

impl BlockToIndex {
    /// Takes care of inserting the data in the database
    pub async fn save(self, db: &DatabaseTransaction) -> Result<(), sea_orm::DbErr> {
        #[cfg(feature = "tracing")]
        tracing::info!(block_height = self.block_info.height, "Indexing block");

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
            block_info: *block,
            txs: vec![],
        });
        action(block_to_index)
    }

    fn find_or_fail(&self, block_height: &u64) -> Result<BlockToIndex, anyhow::Error> {
        let blocks = self.blocks.lock().expect("Can't lock blocks");

        let block_to_index = match blocks.get(block_height) {
            Some(block_to_index) => block_to_index,
            None => anyhow::bail!("Block {} not found", block_height),
        };
        Ok(block_to_index.clone())
    }

    fn remove_or_fail(
        blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
        block_height: &u64,
    ) -> Result<BlockToIndex, anyhow::Error> {
        let mut blocks = blocks.lock().expect("Can't lock blocks");
        let block_to_index = match blocks.remove_entry(block_height) {
            Some(block) => block,
            None => anyhow::bail!("Block {} not found", block_height),
        };
        #[cfg(feature = "tracing")]
        tracing::debug!(
            block_height = block_height,
            blocks_len = blocks.len(),
            "remove_or_fail called"
        );
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
            indexing: true,
        })
    }

    fn start(&self) -> Result<(), anyhow::Error> {
        self.runtime
            .block_on(async { self.context.migrate_db().await })?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        // Avoid running this twice when called manually and from `Drop`
        if !self.indexing {
            return Ok(());
        }
        self.indexing = false;

        // NOTE: This is to allow the indexer to commit all db transactions since this is done
        // async
        for _ in 0..10 {
            let blocks = self.blocks.lock().expect("Can't lock blocks");
            if blocks.is_empty() {
                break;
            }

            drop(blocks);

            sleep(Duration::from_millis(10));
        }

        #[cfg(feature = "tracing")]
        {
            let blocks = self.blocks.lock().expect("Can't lock blocks");
            if !blocks.is_empty() {
                tracing::warn!("Some blocks are still being indexed, maybe non_blocking_indexer `post_indexing` wasn't called by the main app?");
            }
        }

        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> Result<(), anyhow::Error> {
        assert!(self.indexing, "Can't index after shutdown");
        Ok(())
    }

    fn index_block(
        &self,
        block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error> {
        assert!(self.indexing, "Can't index after shutdown");

        #[cfg(feature = "tracing")]
        tracing::info!(block_height = block.height, "index_block called");
        self.find_or_create(block, |_block_to_index| {
            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.height, "index_block started/finished");
            Ok(())
        })
    }

    fn index_transaction(
        &self,
        block: &BlockInfo,
        tx: &Tx,
        tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error> {
        assert!(self.indexing, "Can't index after shutdown");

        #[cfg(feature = "tracing")]
        tracing::info!(block_height = block.height, "index_transaction called");

        self.find_or_create(block, |block_to_index| {
            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.height, "index_transaction started");

            block_to_index.txs.push((tx.clone(), tx_outcome.clone()));

            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.height, "index_transaction finished");
            Ok(())
        })
    }

    fn post_indexing(&self, block_height: u64) -> Result<(), anyhow::Error> {
        assert!(self.indexing, "Can't index after shutdown");

        #[cfg(feature = "tracing")]
        tracing::info!(block_height = block_height, "post_indexing called");

        let context = self.context.clone();
        let block_to_index = self.find_or_fail(&block_height)?;
        let blocks = self.blocks.clone();

        // NOTE: I can't remove the block to index *before* indexing it with DB txn committed, or
        // the shutdown method could be called and see no current block being indexed, and quit.
        // The block would then not be indexed.

        self.runtime.spawn(async move {
            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block_height, "post_indexing started");

            let db = context.db.begin().await?;
            let block_height = block_to_index.block_info.height;
            block_to_index.save(&db).await?;
            db.commit().await?;

            let _ = Self::remove_or_fail(blocks, &block_height)?;
            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block_height, "post_indexing finished");

            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        // If the DatabaseTransactions are left open (not committed) its `Drop` implementation
        // expects a Tokio context. We must call `commit` manually on it within our Tokio
        // context.
        self.shutdown().expect("Can't shutdown indexer");
    }
}
