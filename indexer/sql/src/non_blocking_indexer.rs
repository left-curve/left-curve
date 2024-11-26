use {
    crate::{active_model::Models, entity},
    grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome},
    indexer_core::{bail, error, Context, IndexerTrait},
    sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, TransactionTrait},
    std::{
        collections::HashMap,
        future::Future,
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
#[derive(Debug, Clone)]
pub struct Indexer {
    pub context: Context,
    pub runtime: Option<Arc<Runtime>>,
    pub handle: tokio::runtime::Handle,
    blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
    // TODO: this should be Arc<> because if this Indexer is cloned all instances should be
    // stopping when it's stopped.
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
        entity::transactions::Entity::insert_many(models.transactions)
            .exec(db)
            .await?;
        entity::messages::Entity::insert_many(models.messages)
            .exec(db)
            .await?;
        entity::events::Entity::insert_many(models.events)
            .exec(db)
            .await?;
        Ok(())
    }
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
            handle: runtime.handle().clone(),
            runtime: Some(runtime),
            blocks: Arc::new(Mutex::new(HashMap::new())),
            indexing: false,
        })
    }

    pub fn new_with_database_url(database_url: &str) -> error::Result<Indexer> {
        let runtime = Arc::new(Builder::new_multi_thread()
                //.worker_threads(4)  // Adjust as needed
                .enable_all()
                .build()
                .unwrap());

        let db = runtime.block_on(async { Context::connect_db_with_url(database_url).await })?;

        Ok(Indexer {
            context: Context { db },
            handle: runtime.handle().clone(),
            runtime: Some(runtime),
            blocks: Arc::new(Mutex::new(HashMap::new())),
            indexing: false,
        })
    }

    pub async fn async_new_with_database_url(
        handle: &tokio::runtime::Handle,
        database_url: &str,
    ) -> error::Result<Indexer> {
        let db = Context::connect_db_with_url(database_url).await?;

        Ok(Indexer {
            context: Context { db },
            handle: handle.clone(),
            runtime: None,
            blocks: Arc::new(Mutex::new(HashMap::new())),
            indexing: false,
        })
    }

    pub fn new_with_handle_and_database_url(
        handle: &tokio::runtime::Handle,
        database_url: &str,
    ) -> error::Result<Indexer> {
        let db = handle.block_on(async { Context::connect_db_with_url(database_url).await })?;

        Ok(Indexer {
            context: Context { db },
            handle: handle.clone(),
            runtime: None,
            blocks: Arc::new(Mutex::new(HashMap::new())),
            indexing: false,
        })
    }

    fn find_or_create<F, R>(&self, block: &BlockInfo, action: F) -> error::Result<R>
    where
        F: FnOnce(&mut BlockToIndex) -> error::Result<R>,
    {
        let mut blocks = self.blocks.lock().expect("Can't lock blocks");
        let block_to_index = blocks.entry(block.height).or_insert(BlockToIndex {
            block_info: *block,
            txs: vec![],
        });
        action(block_to_index)
    }

    fn find_or_fail(&self, block_height: &u64) -> error::Result<BlockToIndex> {
        let blocks = self.blocks.lock().expect("Can't lock blocks");

        let block_to_index = match blocks.get(block_height) {
            Some(block_to_index) => block_to_index,
            None => bail!("Block {} not found", block_height),
        };
        Ok(block_to_index.clone())
    }

    fn remove_or_fail(
        blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
        block_height: &u64,
    ) -> error::Result<BlockToIndex> {
        let mut blocks = blocks.lock().expect("Can't lock blocks");
        let block_to_index = match blocks.remove_entry(block_height) {
            Some(block) => block,
            None => indexer_core::bail!("Block {} not found", block_height),
        };
        #[cfg(feature = "tracing")]
        tracing::debug!(
            block_height = block_height,
            blocks_len = blocks.len(),
            "remove_or_fail called"
        );
        Ok(block_to_index.1)
    }

    /// Code in the indexer is running without async context (within Grug) and with an async
    /// context (Dango). This is to ensure it works in both cases.
    /// NOTE: The Tokio runtime *must* be multi-threaded:
    /// - #[tokio::main]
    /// - #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    fn call<F>(&self, closure: F) -> error::Result<()>
    where
        F: Future<Output = Result<(), indexer_core::error::Error>> + Send + 'static,
    {
        match self.runtime.as_ref() {
            Some(runtime) => {
                runtime.block_on(closure)?;
            },
            None => {
                // let handle = self.handle.clone();
                // println!("Spawn blocking");
                // let join_handle = self.handle.spawn_blocking(move || {
                //    println!("Spawn blocking closure");
                //    handle.block_on(closure)?;
                //    println!("Spawn blocking closure finished");
                //    Ok::<(), error::Error>(())
                //});
                //
                //// Use `block_on` to wait for the `spawn_blocking` task synchronously
                // self.handle.block_on(async { join_handle.await? })?;

                tokio::task::block_in_place(|| self.handle.block_on(closure))?;
            },
        }

        Ok(())
    }
}

impl IndexerTrait for Indexer {
    fn start(&mut self) -> error::Result<()> {
        let context = self.context.clone();
        self.call(async move {
            context.migrate_db().await.expect("Can't run migration");
            Ok(())
        })?;

        self.indexing = true;
        Ok(())
    }

    fn shutdown(&mut self) -> error::Result<()> {
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

    fn pre_indexing(&self, _block_height: u64) -> error::Result<()> {
        assert!(self.indexing, "Can't index after shutdown");
        Ok(())
    }

    fn index_block(&self, block: &BlockInfo, _block_outcome: &BlockOutcome) -> error::Result<()> {
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
    ) -> error::Result<()> {
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

    fn post_indexing(&self, block_height: u64) -> error::Result<()> {
        assert!(self.indexing, "Can't index after shutdown");

        #[cfg(feature = "tracing")]
        tracing::info!(block_height = block_height, "post_indexing called");

        let context = self.context.clone();
        let block_to_index = self.find_or_fail(&block_height)?;
        let blocks = self.blocks.clone();

        // NOTE: I can't remove the block to index *before* indexing it with DB txn committed, or
        // the shutdown method could be called and see no current block being indexed, and quit.
        // The block would then not be indexed.

        self.handle.spawn(async move {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// This is when used from Dango, which is async. In such case `App` does not have its own
    /// Tokio runtime and use the main handler. Making sure `start` can be called in an async
    /// context.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_start() -> anyhow::Result<()> {
        let handle = tokio::runtime::Handle::current();

        let mut app = Indexer::async_new_with_database_url(&handle, "sqlite::memory:")
            .await
            .expect("Can't create indexer");
        assert!(!app.indexing);
        app.start().expect("Can't start Indexer");
        assert!(app.indexing);
        app.shutdown().expect("Can't shutdown Indexer");
        assert!(!app.indexing);

        Ok(())
    }
}
