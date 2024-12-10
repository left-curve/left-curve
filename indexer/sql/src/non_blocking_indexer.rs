use {
    crate::{bail, block::BlockToIndex, entity, error, indexer_path::IndexerPath, Context},
    glob::glob,
    grug_app::{Indexer, LAST_FINALIZED_BLOCK},
    grug_types::{
        BlockInfo, BlockOutcome, Defined, MaybeDefined, Storage, Tx, TxOutcome, Undefined,
    },
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait},
    std::{
        collections::HashMap,
        future::Future,
        ops::Deref,
        path::PathBuf,
        sync::{Arc, Mutex},
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::{Builder, Handle, Runtime},
};

// ------------------------------- IndexerBuilder ------------------------------

pub struct IndexerBuilder<DB = Undefined<String>, P = Undefined<IndexerPath>> {
    handle: RuntimeHandler,
    db_url: DB,
    indexer_path: P,
}

impl Default for IndexerBuilder {
    fn default() -> Self {
        Self {
            handle: RuntimeHandler::default(),
            db_url: Undefined::default(),
            indexer_path: Undefined::default(),
        }
    }
}

impl<P> IndexerBuilder<Undefined<String>, P> {
    pub fn with_database_url<URL>(self, db_url: URL) -> IndexerBuilder<Defined<String>, P>
    where
        URL: ToString,
    {
        IndexerBuilder {
            handle: self.handle,
            indexer_path: self.indexer_path,
            db_url: Defined::new(db_url.to_string()),
        }
    }

    pub fn with_memory_database(self) -> IndexerBuilder<Defined<String>, P> {
        self.with_database_url("sqlite::memory:")
    }
}

impl<DB> IndexerBuilder<DB, Undefined<IndexerPath>> {
    pub fn with_tmpdir(self) -> IndexerBuilder<DB, Defined<IndexerPath>> {
        IndexerBuilder {
            handle: self.handle,
            indexer_path: Defined::new(IndexerPath::default()),
            db_url: self.db_url,
        }
    }

    pub fn with_dir(self, dir: PathBuf) -> IndexerBuilder<DB, Defined<IndexerPath>> {
        IndexerBuilder {
            handle: self.handle,
            indexer_path: Defined::new(IndexerPath::Dir(dir)),
            db_url: self.db_url,
        }
    }
}

impl<DB, P> IndexerBuilder<DB, P>
where
    DB: MaybeDefined<String>,
    P: MaybeDefined<IndexerPath>,
{
    pub fn build(self) -> error::Result<NonBlockingIndexer> {
        let db = match self.db_url.maybe_into_inner() {
            Some(url) => self
                .handle
                .block_on(async { Context::connect_db_with_url(&url).await }),
            None => self.handle.block_on(async { Context::connect_db().await }),
        }?;

        let indexer_path = self.indexer_path.maybe_into_inner().unwrap_or_default();
        indexer_path.create_dirs_if_needed()?;

        Ok(NonBlockingIndexer {
            indexer_path,
            context: Context { db },
            handle: self.handle,
            blocks: Default::default(), // Arc::new(Mutex::new(HashMap::new())),
            indexing: false,
        })
    }
}

// ----------------------------- NonBlockingIndexer ----------------------------

/// Because I'm using `.spawn` in this implementation, I ran into lifetime issues where I need the
/// data to live as long as the spawned task.
///
/// I also have potential issues where the task spawned in `pre_indexing` to create a DB
/// transaction (in the sync implentation of this trait) could be theorically executed after the
/// task spawned in `index_block` and `index_transaction` meaning I'd have to check in these
/// functions if the transaction exists or not.
///
/// Decided to do different and prepare the data in memory in `blocks` to inject all data in a single Tokio
/// spawned task
#[derive(Debug)]
pub struct NonBlockingIndexer {
    indexer_path: IndexerPath,
    pub context: Context,
    pub handle: RuntimeHandler,
    blocks: Arc<Mutex<HashMap<u64, BlockToIndex>>>,
    pub indexing: bool,
}

impl NonBlockingIndexer {
    /// Look in memory for a block to be indexed, or create a new one
    fn find_or_create<F, R>(
        &self,
        block: &BlockInfo,
        block_outcome: Option<&BlockOutcome>,
        action: F,
    ) -> error::Result<R>
    where
        F: FnOnce(&mut BlockToIndex) -> error::Result<R>,
    {
        let mut blocks = self.blocks.lock().expect("Can't lock blocks");
        let block_to_index = blocks.entry(block.height).or_insert(BlockToIndex::new(
            *block,
            None,
            self.block_tmp_filename(block.height)
                .to_string_lossy()
                .to_string(),
        ));

        if block_outcome.is_some() {
            tracing::warn!("Block outcome is missing, SETTING");
            block_to_index.block_outcome = block_outcome.cloned();
        }

        action(block_to_index)
    }

    /// Look in memory for a block to be indexed, or fail if not found
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

    /// Look in memory for a block to be removed, or fail if not found
    pub fn remove_or_fail(
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

    /// Wait for all blocks to be indexed
    pub fn wait_for_finish(&self) {
        for _ in 0..10 {
            if self.blocks.lock().expect("Can't lock blocks").is_empty() {
                break;
            }

            sleep(Duration::from_millis(10));
        }
    }
}

// ------------------------------- DB Related ----------------------------------

impl NonBlockingIndexer {
    /// Delete a block and its related content from the database
    pub fn delete_block_from_db(&self, block_height: u64) -> error::Result<()> {
        self.handle.block_on(async move {
            let db = self.context.db.begin().await?;

            entity::blocks::Entity::delete_many()
                .filter(entity::blocks::Column::BlockHeight.eq(block_height))
                .exec(&db)
                .await?;

            entity::transactions::Entity::delete_many()
                .filter(entity::transactions::Column::BlockHeight.eq(block_height))
                .exec(&db)
                .await?;

            entity::messages::Entity::delete_many()
                .filter(entity::messages::Column::BlockHeight.eq(block_height))
                .exec(&db)
                .await?;

            entity::events::Entity::delete_many()
                .filter(entity::events::Column::BlockHeight.eq(block_height))
                .exec(&db)
                .await?;

            db.commit().await?;

            Ok::<(), sea_orm::error::DbErr>(())
        })?;
        Ok(())
    }
}

impl NonBlockingIndexer {
    /// Where will this block be temporarily saved on disk
    pub fn block_tmp_filename(&self, block_height: u64) -> PathBuf {
        // Using a specific namespace `block-` to avoid conflicts with other files (tmpfile)
        let filename = format!("block-{block_height}");
        self.indexer_path.tmp_path().join(filename)
    }

    /// Load all existing tmp files and ensure they've been indexed
    fn load_all_from_disk(&self, latest_block_height: u64) -> error::Result<()> {
        // You're not supposed to have many remaining files, probably at most 1 file in rare case
        // of process crash. I'll load each file one after the other, I dont need to use
        // multi-thread to go faster.

        let pattern = format!("{}/block-*", self.indexer_path.tmp_path().to_string_lossy());
        for file in glob(&pattern)? {
            match file {
                Ok(path) => match BlockToIndex::load_tmp_file(&path) {
                    Ok(block_to_index) => {
                        #[cfg(feature = "tracing")]
                        tracing::info!(
                            block_height = block_to_index.block_info.height,
                            "load_all_from_disk filename started"
                        );

                        // This block is higher than the current latest block. This can happen when
                        // a crash occured after `do_finalize_block` has been called, but before
                        // `do_commit` was called.
                        if block_to_index.block_info.height > latest_block_height {
                            block_to_index.delete_tmp_file()?;
                            continue;
                        }

                        self.handle.block_on(async {
                            let db = self.context.db.begin().await?;
                            block_to_index.save(&db).await?;
                            db.commit().await?;
                            Ok::<(), error::IndexerError>(())
                        })?;

                        block_to_index.delete_tmp_file()?;

                        #[cfg(feature = "tracing")]
                        tracing::info!(
                            block_height = block_to_index.block_info.height,
                            "load_all_from_disk filename finished"
                        );
                    },
                    Err(_err) => {
                        #[cfg(feature = "tracing")]
                        tracing::error!(error = %_err, path = %path.to_string_lossy(), "can't load block from tmp_file");
                    },
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_err, "can't look at filename");
                },
            }
        }

        Ok(())
    }
}

impl Indexer for NonBlockingIndexer {
    type Error = crate::error::IndexerError;

    fn start<S>(&mut self, storage: &S) -> error::Result<()>
    where
        S: Storage,
    {
        self.handle
            .block_on(async { self.context.migrate_db().await })?;

        match LAST_FINALIZED_BLOCK.load(storage) {
            Err(_err) => {
                // This happens when the chain starts at genesis
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_err, "No LAST_FINALIZED_BLOCK found");
            },
            Ok(block) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    block_height = block.height,
                    "start called, found a previous block"
                );
                self.load_all_from_disk(block.height)?;
            },
        }

        // Save on indexer DB all blocks that were indexed on disk but not saved

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

    /// NOTE: `index_block` is called *after* `index_transaction`
    fn index_block(&self, block: &BlockInfo, block_outcome: &BlockOutcome) -> error::Result<()> {
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height = block.height, "index_block called");

        self.find_or_create(block, Some(block_outcome), |block_to_index| {
            #[cfg(feature = "tracing")]
            tracing::debug!(block_height = block.height, "index_block started");

            block_to_index.save_tmp_file()?;

            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.height, "index_block finished");
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
        tracing::debug!(block_height = block.height, "index_transaction called");

        self.find_or_create(block, None, |block_to_index| {
            #[cfg(feature = "tracing")]
            tracing::debug!(block_height = block.height, "index_transaction started");

            block_to_index.txs.push((tx, tx_outcome));

            #[cfg(feature = "tracing")]
            tracing::debug!(block_height = block.height, "index_transaction finished");

            Ok(())
        })
    }

    fn post_indexing(&self, block_height: u64) -> error::Result<()> {
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height = block_height, "post_indexing called");

        let context = self.context.clone();
        let block_to_index = self.find_or_fail(block_height)?;
        let blocks = self.blocks.clone();

        // NOTE: I can't remove the block to index *before* indexing it with DB txn committed, or
        // the shutdown method could be called and see no current block being indexed, and quit.
        // The block would then not be indexed.

        self.handle.spawn(async move {
            #[cfg(feature = "tracing")]
            tracing::debug!(block_height = block_height, "post_indexing started");

            let db = context.db.begin().await?;
            let block_height = block_to_index.block_info.height;
            block_to_index.save(&db).await?;
            db.commit().await?;

            block_to_index.delete_tmp_file()?;

            Self::remove_or_fail(blocks, &block_height)?;

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

// ------------------------------- RuntimeHandler ------------------------------

/// Wrapper around Tokio runtime to allow running in sync context
#[derive(Debug)]
pub struct RuntimeHandler {
    runtime: Option<Runtime>,
    handle: Handle,
}

/// Derive macro is not working because generics.
impl Default for RuntimeHandler {
    fn default() -> Self {
        let (runtime, handle) = match Handle::try_current() {
            Ok(handle) => (None, handle),
            Err(_) => {
                let runtime = Builder::new_multi_thread().enable_all().build().unwrap();
                let handle = runtime.handle().clone();
                (Some(runtime), handle)
            },
        };
        Self { runtime, handle }
    }
}

impl Deref for RuntimeHandler {
    type Target = Handle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl RuntimeHandler {
    /// Runs a future in the Tokio runtime, blocking the current thread until the future is resolved.
    ///
    /// This function override [`Handle::block_on`] to allow running in a sync context,
    /// because [`RuntimeHandler`] implements [`Deref`] to [`Handle`].
    ///
    /// Code in the indexer is running without async context (within Grug) and with an async
    /// context (Dango). This is to ensure it works in both cases.
    ///
    /// NOTE: The Tokio runtime *must* be multi-threaded with either:
    /// - `#[tokio::main]`
    /// - `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`
    pub fn block_on<F, R>(&self, closure: F) -> R
    where
        F: Future<Output = R>,
    {
        if self.runtime.is_some() {
            self.handle.block_on(closure)
        } else {
            tokio::task::block_in_place(|| self.handle.block_on(closure))
        }
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
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("failed to set global tracing subscriber");

        let mut indexer = IndexerBuilder::default()
            .with_memory_database()
            .with_tmpdir()
            .build()?;
        let storage = MockStorage::new();

        assert!(!indexer.indexing);
        indexer.start(&storage).expect("Can't start Indexer");
        assert!(indexer.indexing);

        indexer.shutdown().expect("Can't shutdown Indexer");
        assert!(!indexer.indexing);

        Ok(())
    }
}
