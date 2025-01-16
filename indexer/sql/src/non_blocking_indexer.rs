use {
    crate::{
        bail, block_to_index::BlockToIndex, entity, error, indexer_path::IndexerPath, Context,
    },
    grug_app::{Indexer, LAST_FINALIZED_BLOCK},
    grug_types::{Block, BlockOutcome, Defined, MaybeDefined, Storage, Undefined},
    sea_orm::TransactionTrait,
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
    keep_blocks: bool,
}

impl Default for IndexerBuilder {
    fn default() -> Self {
        Self {
            handle: RuntimeHandler::default(),
            db_url: Undefined::default(),
            indexer_path: Undefined::default(),
            keep_blocks: false,
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
            keep_blocks: self.keep_blocks,
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
            keep_blocks: self.keep_blocks,
        }
    }

    pub fn with_dir(self, dir: PathBuf) -> IndexerBuilder<DB, Defined<IndexerPath>> {
        IndexerBuilder {
            handle: self.handle,
            indexer_path: Defined::new(IndexerPath::Dir(dir)),
            db_url: self.db_url,
            keep_blocks: self.keep_blocks,
        }
    }
}

impl<DB, P> IndexerBuilder<DB, P>
where
    DB: MaybeDefined<String>,
    P: MaybeDefined<IndexerPath>,
{
    /// If true, the block/block_outcome used by the indexer will be kept on disk after being
    /// indexed. This is useful for reruning the indexer since genesis if code is changing, and
    /// we'll want to run at least one node with this enabled and sync those on S3.
    pub fn with_keep_blocks(self, keep_blocks: bool) -> Self {
        Self {
            handle: self.handle,
            db_url: self.db_url,
            indexer_path: self.indexer_path,
            keep_blocks,
        }
    }

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
            blocks: Default::default(),
            indexing: false,
            keep_blocks: self.keep_blocks,
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
    // NOTE: this could be Arc<AtomicBool> because if this Indexer is cloned all instances should
    // be stopping when the program is stopped, but then it adds a lot of boilerplate. So far, code
    // as I understand it doesn't clone `App` in a way it'd raise concern.
    pub indexing: bool,
    keep_blocks: bool,
}

impl NonBlockingIndexer {
    /// Look in memory for a block to be indexed, or create a new one
    fn find_or_create<F, R>(
        &self,
        block_filename: PathBuf,
        block: &Block,
        block_outcome: &BlockOutcome,
        action: F,
    ) -> error::Result<R>
    where
        F: FnOnce(&mut BlockToIndex) -> error::Result<R>,
    {
        let mut blocks = self.blocks.lock().expect("Can't lock blocks");
        let block_to_index = blocks.entry(block.info.height).or_insert(BlockToIndex::new(
            block_filename,
            block.clone(),
            block_outcome.clone(),
        ));

        block_to_index.block_outcome = block_outcome.clone();

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
        for _ in 0..100 {
            if self.blocks.lock().expect("Can't lock blocks").is_empty() {
                break;
            }

            sleep(Duration::from_millis(100));
        }
    }
}

// ------------------------------- DB Related ----------------------------------

impl NonBlockingIndexer {
    /// Delete a block and its related content from the database
    pub fn delete_block_from_db(&self, block_height: u64) -> error::Result<()> {
        self.handle.block_on(async move {
            let db = self.context.db.begin().await?;
            entity::blocks::Entity::delete_block_and_data(&db, block_height).await?;
            db.commit().await?;

            Ok::<(), sea_orm::error::DbErr>(())
        })?;
        Ok(())
    }
}

impl NonBlockingIndexer {
    /// Where will this block be temporarily saved on disk
    pub fn block_filename(&self, block_height: u64) -> PathBuf {
        let directory = self.indexer_path.block_path();
        let filename = block_height.to_string();
        directory.join(filename)
    }

    /// Index all previous blocks not yet indexed.
    fn index_previous_unindexed_blocks(&self, latest_block_height: u64) -> error::Result<()> {
        let last_indexed_block_height = self.handle.block_on(async {
            entity::blocks::Entity::find_last_block_height(&self.context.db).await
        })?;

        let last_indexed_block_height = match last_indexed_block_height {
            Some(height) => height as u64,
            None => 1, // happens when you index since genesis
        };

        for block_height in last_indexed_block_height..=latest_block_height {
            let block_filename = self.block_filename(block_height);

            let block_to_index = BlockToIndex::load_from_disk(block_filename.clone()).unwrap_or_else(|err| {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %err, block_height, "can't load block from disk");
                    panic!("can't load block from disk, can't continue as you'd be missing indexed blocks");
            });

            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = block_height,
                "index_previous_unindexed_blocks started"
            );

            self.handle.block_on(async {
                let db = self.context.db.begin().await?;
                block_to_index.save(&db).await?;
                db.commit().await?;
                Ok::<(), error::IndexerError>(())
            })?;

            if !self.keep_blocks {
                if let Err(_err) = BlockToIndex::delete_from_disk(block_filename.clone()) {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_err, block_height, block_filename = %block_filename.display(), "can't delete block from disk");
                }
            }

            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = block_height,
                "index_previous_unindexed_blocks ended"
            );
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
                self.index_previous_unindexed_blocks(block.height)?;
            },
        }

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

    fn index_block(&self, block: &Block, block_outcome: &BlockOutcome) -> error::Result<()> {
        if !self.indexing {
            bail!("Can't index after shutdown");
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height = block.info.height, "index_block called");

        let block_filename = self.block_filename(block.info.height);

        self.find_or_create(block_filename, block, block_outcome, |block_to_index| {
            #[cfg(feature = "tracing")]
            tracing::debug!(block_height = block.info.height, "index_block started");

            block_to_index.save_to_disk()?;

            #[cfg(feature = "tracing")]
            tracing::info!(block_height = block.info.height, "index_block finished");
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
        let keep_blocks = self.keep_blocks;
        let block_filename = self.block_filename(block_to_index.block.info.height);

        // NOTE: I can't remove the block to index *before* indexing it with DB txn committed, or
        // the shutdown method could be called and see no current block being indexed, and quit.
        // The block would then not be indexed.
        self.handle.spawn(async move {
            #[cfg(feature = "tracing")]
            tracing::debug!(block_height = block_height, "post_indexing started");

            let db = context.db.begin().await?;
            let block_height = block_to_index.block.info.height;
            if let Err(_err) = block_to_index.save(&db).await {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %_err, "can't save to db in post_indexing");
            }
            db.commit().await?;

            if !keep_blocks {
                if let Err(_err) = BlockToIndex::delete_from_disk(block_filename.clone()) {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_err, block_filename = %block_filename.display(), "can't delete block from disk in post_indexing");
                }
            } else {
                // compress takes CPU, so we do it in a spawned blocking task
                tokio::task::spawn_blocking(|| async move {
                    if let Err(_err) = BlockToIndex::compress_file(block_filename.clone()) {
                        #[cfg(feature = "tracing")]
                        tracing::error!(error = %_err, block_filename = %block_filename.display(), "can't compress block on disk in post_indexing");
                    }
                });
            }

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
