use {
    crate::{active_model::Models, bail, entity, error, Context},
    grug_app::{Indexer, LAST_FINALIZED_BLOCK},
    grug_types::{
        BlockInfo, BlockOutcome, Defined, MaybeDefined, Storage, Tx, TxOutcome, Undefined,
    },
    sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, TransactionTrait},
    std::{
        collections::HashMap,
        future::Future,
        sync::{Arc, Mutex},
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::{Builder, Handle, Runtime},
};

pub struct IndexerBuilder<DB = Undefined<String>> {
    runtime: Option<Arc<Runtime>>,
    handle: tokio::runtime::Handle,
    db_url: DB,
}

impl Default for IndexerBuilder {
    fn default() -> IndexerBuilder {
        let (runtime, handle) = match Handle::try_current() {
            Ok(handle) => (None, handle),
            Err(_) => {
                let runtime = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());
                let handle = runtime.handle().clone();
                (Some(runtime), handle)
            },
        };

        IndexerBuilder {
            runtime,
            handle,
            db_url: Undefined::default(),
        }
    }
}

impl IndexerBuilder {
    pub fn with_database_url<URL>(self, db_url: URL) -> IndexerBuilder<Defined<String>>
    where
        URL: ToString,
    {
        IndexerBuilder {
            runtime: self.runtime,
            handle: self.handle,
            db_url: Defined::new(db_url.to_string()),
        }
    }

    pub fn with_memory_database(self) -> IndexerBuilder<Defined<String>> {
        self.with_database_url("sqlite::memory:")
    }
}

impl<DB> IndexerBuilder<DB>
where
    DB: MaybeDefined<String>,
{
    pub fn build(self) -> error::Result<NonBlockingIndexer> {
        let db = match self.db_url.maybe_into_inner() {
            Some(url) => block_call(self.runtime.as_ref(), &self.handle, async {
                Context::connect_db_with_url(&url).await
            }),
            None => block_call(self.runtime.as_ref(), &self.handle, async {
                Context::connect_db().await
            }),
        }?;

        Ok(NonBlockingIndexer {
            context: Context { db },
            handle: self.handle,
            runtime: self.runtime,
            blocks: Arc::new(Mutex::new(HashMap::new())),
            indexing: false,
        })
    }
}

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
pub struct NonBlockingIndexer {
    pub context: Context,
    pub runtime: Option<Arc<Runtime>>,
    pub handle: tokio::runtime::Handle,
    blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
    // NOTE: this could be Arc<AtomicBool> because if this Indexer is cloned all instances should
    // be stopping when the program is stopped, but then it adds a lot of boilerplate. So far, code
    // as I understand it doesn't clone `App` in a way it'd raise concern.
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
    pub async fn save(self, db: &DatabaseTransaction) -> error::Result<()> {
        #[cfg(feature = "tracing")]
        tracing::info!(block_height = self.block_info.height, "Indexing block");

        let mut models = Models::build(&self.block_info)?;
        for (tx, tx_outcome) in self.txs.into_iter() {
            models.push(tx, tx_outcome)?;
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

impl NonBlockingIndexer {
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

    fn find_or_fail(&self, block_height: u64) -> error::Result<BlockToIndex> {
        let blocks = self.blocks.lock().expect("Can't lock blocks");

        let block_to_index = match blocks.get(&block_height) {
            Some(block_to_index) => block_to_index,
            None => {
                bail!("Block {} not found", block_height);
            },
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
            None => {
                bail!("Block {} not found", block_height);
            },
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

impl NonBlockingIndexer {
    /// Save this block on disk to ensure it'll be indexed when process is crashed
    fn save_on_disk(&self, block_height: u64) -> error::Result<()> {
        let blocks = self.blocks.lock().expect("Can't lock blocks");
        if let Some(block) = blocks.get(&block_height).cloned() {
            drop(blocks);

            // TODO: 1. serialize block 2. write to disk
            // use tempfile and `mv` for atomic and ensure file isn't half written
        }
        Ok(())
    }

    // TODO: run once when `start` is called
    fn load_all_from_disk(&self, latest_block_height: u64) -> error::Result<()> {
        // TODO: look at the local cache and ensure all blocks were indexed, delete anything with
        // higher number than latest_block_height
        // fetch latest block_height from indexer DB
        todo!()
    }

    // TODO: remove the on-disk cache, block was indexed
    fn delete_from_disk(&self, block_height: u64) -> error::Result<()> {
        todo!();
    }
}

impl Indexer for NonBlockingIndexer {
    type Error = crate::error::IndexerError;

    fn start<S>(&mut self, storage: &S) -> error::Result<()>
    where
        S: Storage,
    {
        block_call(self.runtime.as_ref(), &self.handle, async {
            self.context.migrate_db().await
        })?;

        let _block = match LAST_FINALIZED_BLOCK.load(storage) {
            Err(_) => {
                #[cfg(feature = "tracing")]
                tracing::warn!("No block found in storage");
                None
            },
            Ok(block) => {
                #[cfg(feature = "tracing")]
                tracing::warn!("block found in storage");
                // TODO: ensure we indexed all previous blocks
                self.load_all_from_disk(block.height)?;
                Some(block)
            },
        };

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
        // async. It just loops quickly making sure no indexing blocks are remaining.
        for _ in 0..10 {
            if self.blocks.lock().expect("Can't lock blocks").is_empty() {
                break;
            }

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
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

        Ok(())
    }

    fn index_block(&self, block: &BlockInfo, _block_outcome: &BlockOutcome) -> error::Result<()> {
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

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
        tx: Tx,
        tx_outcome: TxOutcome,
    ) -> error::Result<()> {
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

        #[cfg(feature = "tracing")]
        tracing::info!(block_height = block.height, "index_transaction called");

        self.find_or_create(block, |block_to_index| {
            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.height, "index_transaction started");

            block_to_index.txs.push((tx, tx_outcome));

            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.height, "index_transaction finished");

            Ok(())
        })
    }

    fn post_indexing(&self, block_height: u64) -> error::Result<()> {
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

        #[cfg(feature = "tracing")]
        tracing::info!(block_height = block_height, "post_indexing called");

        let context = self.context.clone();
        let block_to_index = self.find_or_fail(block_height)?;
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

            Ok::<_, error::IndexerError>(())
        });

        Ok(())
    }
}

impl Drop for NonBlockingIndexer {
    fn drop(&mut self) {
        // If the DatabaseTransactions are left open (not committed) its `Drop` implementation
        // expects a Tokio context. We must call `commit` manually on it within our Tokio
        // context.
        self.shutdown().expect("Can't shutdown indexer");
    }
}

/// Code in the indexer is running without async context (within Grug) and with an async
/// context (Dango). This is to ensure it works in both cases.
/// NOTE: The Tokio runtime *must* be multi-threaded with either:
/// - #[tokio::main]
/// - #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
fn block_call<F, R>(runtime: Option<&Arc<Runtime>>, handle: &Handle, closure: F) -> R
where
    F: Future<Output = R>,
{
    match runtime.as_ref() {
        Some(runtime) => runtime.block_on(closure),
        None => tokio::task::block_in_place(|| handle.block_on(closure)),
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::MockStorage};

    /// This is when used from Dango, which is async. In such case the indexer does not have its
    /// own Tokio runtime and use the main handler. Making sure `start` can be called in an async
    /// context.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_start() -> anyhow::Result<()> {
        let mut indexer = IndexerBuilder::default().with_memory_database().build()?;
        let storage = MockStorage::new();

        assert!(!indexer.indexing);
        indexer.start(&storage).expect("Can't start Indexer");
        assert!(indexer.indexing);

        indexer.shutdown().expect("Can't shutdown Indexer");
        assert!(!indexer.indexing);

        Ok(())
    }
}
