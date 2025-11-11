use {
    crate::{
        Context, EventCache, EventCacheWriter, bail,
        block_to_index::BlockToIndex,
        entity,
        error::{self, IndexerError},
        indexer_path::IndexerPath,
        pubsub::{MemoryPubSub, PostgresPubSub, PubSubType},
    },
    grug_app::{Indexer as IndexerTrait, LAST_FINALIZED_BLOCK},
    grug_types::{
        Block, BlockAndBlockOutcomeWithHttpDetails, BlockOutcome, Defined, HttpRequestDetails,
        MaybeDefined, Storage, Undefined,
    },
    sea_orm::DatabaseConnection,
    std::{
        collections::HashMap,
        future::Future,
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicU64, Ordering},
        },
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::{Builder, Handle, Runtime},
};

// ------------------------------- IndexerBuilder ------------------------------

pub struct IndexerBuilder<DB = Undefined<String>> {
    handle: RuntimeHandler,
    db_url: DB,
    db_max_connections: u32,
    pubsub: PubSubType,
    event_cache_window: usize,
}

impl Default for IndexerBuilder {
    fn default() -> Self {
        Self {
            handle: RuntimeHandler::default(),
            db_url: Undefined::default(),
            db_max_connections: 10,
            pubsub: PubSubType::Memory,
            event_cache_window: 100,
        }
    }
}

impl IndexerBuilder<Defined<String>> {
    pub fn with_database_max_connections(self, db_max_connections: u32) -> Self {
        IndexerBuilder {
            handle: self.handle,
            db_url: self.db_url,
            db_max_connections,
            pubsub: self.pubsub,
            event_cache_window: self.event_cache_window,
        }
    }
}

impl IndexerBuilder<Undefined<String>> {
    pub fn with_database_url<URL>(self, db_url: URL) -> IndexerBuilder<Defined<String>>
    where
        URL: ToString,
    {
        IndexerBuilder {
            handle: self.handle,
            db_url: Defined::new(db_url.to_string()),
            db_max_connections: self.db_max_connections,
            pubsub: self.pubsub,
            event_cache_window: self.event_cache_window,
        }
    }

    pub fn with_memory_database(self) -> IndexerBuilder<Defined<String>> {
        self.with_database_url("sqlite::memory:")
    }
}

// impl<DB> IndexerBuilder<DB, Undefined<IndexerPath>> {
//     pub fn with_tmpdir(self) -> IndexerBuilder<DB, Defined<IndexerPath>> {
//         IndexerBuilder {
//             handle: self.handle,
//             indexer_path: Defined::new(IndexerPath::default()),
//             db_url: self.db_url,
//             db_max_connections: self.db_max_connections,
//             pubsub: self.pubsub,
//             event_cache_window: self.event_cache_window,
//         }
//     }

//     pub fn with_dir(self, dir: PathBuf) -> IndexerBuilder<DB, Defined<IndexerPath>> {
//         IndexerBuilder {
//             handle: self.handle,
//             indexer_path: Defined::new(IndexerPath::Dir(dir)),
//             db_url: self.db_url,
//             db_max_connections: self.db_max_connections,
//             keep_blocks: self.keep_blocks,
//             pubsub: self.pubsub,
//             event_cache_window: self.event_cache_window,
//         }
//     }
// }

impl<DB> IndexerBuilder<DB> {
    pub fn with_sqlx_pubsub(self) -> IndexerBuilder<DB> {
        IndexerBuilder {
            handle: self.handle,
            db_url: self.db_url,
            db_max_connections: self.db_max_connections,
            pubsub: PubSubType::Postgres,
            event_cache_window: self.event_cache_window,
        }
    }
}

impl<DB> IndexerBuilder<DB>
where
    DB: MaybeDefined<String>,
{
    pub fn with_event_cache_window(self, event_cache_window: usize) -> Self {
        Self {
            handle: self.handle,
            db_url: self.db_url,
            db_max_connections: self.db_max_connections,
            pubsub: self.pubsub,
            event_cache_window,
        }
    }

    pub fn build_context(self) -> error::Result<Context> {
        let db = match self.db_url.maybe_into_inner() {
            Some(url) => self.handle.block_on(async {
                Context::connect_db_with_url(&url, self.db_max_connections).await
            }),
            None => self.handle.block_on(async { Context::connect_db().await }),
        }?;

        let mut context = Context {
            db: db.clone(),
            // This gets overwritten in the next match
            pubsub: Arc::new(MemoryPubSub::new(100)),
            event_cache: EventCache::new(self.event_cache_window),
        };

        match self.pubsub {
            PubSubType::Postgres => self.handle.block_on(async {
                if let DatabaseConnection::SqlxPostgresPoolConnection(_) = db {
                    let pool: &sqlx::PgPool = db.get_postgres_connection_pool();

                    context.pubsub = Arc::new(PostgresPubSub::new(pool.clone(), "blocks").await?);
                }

                Ok::<(), IndexerError>(())
            })?,
            PubSubType::Memory => {},
        }

        Ok(context)
    }

    pub fn build(self) -> error::Result<Indexer> {
        let db = match self.db_url.maybe_into_inner() {
            Some(url) => self.handle.block_on(async {
                Context::connect_db_with_url(&url, self.db_max_connections).await
            }),
            None => self.handle.block_on(async { Context::connect_db().await }),
        }?;

        // Generate unique ID
        let id = INDEXER_COUNTER.fetch_add(1, Ordering::SeqCst);

        let mut context = Context {
            db: db.clone(),
            // This gets overwritten in the next match
            pubsub: Arc::new(MemoryPubSub::new(100)),
            event_cache: EventCache::new(self.event_cache_window),
        };

        match self.pubsub {
            PubSubType::Postgres => self.handle.block_on(async {
                if let DatabaseConnection::SqlxPostgresPoolConnection(_) = db {
                    let pool: &sqlx::PgPool = db.get_postgres_connection_pool();

                    context.pubsub = Arc::new(PostgresPubSub::new(pool.clone(), "blocks").await?);
                }

                Ok::<(), IndexerError>(())
            })?,
            PubSubType::Memory => {},
        }

        Ok(Indexer {
            context,
            handle: self.handle,
            indexing: false,
            id,
            indexing_blocks: Default::default(),
        })
    }
}

// ----------------------------- NonBlockingIndexer ----------------------------

// Add a global counter for unique IDs
static INDEXER_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Because I'm using `.spawn` in this implementation, I ran into lifetime issues where I need the
/// data to live as long as the spawned task.
///
/// I also have potential issues where the task spawned in `pre_indexing` to create a DB
/// transaction (in the sync implmentation of this trait) could be theorically executed after the
/// task spawned in `index_block` and `index_transaction` meaning I'd have to check in these
/// functions if the transaction exists or not.
///
/// Decided to do different and prepare the data in memory in `blocks` to inject all data in a single Tokio
/// spawned task
pub struct Indexer {
    pub context: Context,
    pub handle: RuntimeHandler,
    // NOTE: this could be Arc<AtomicBool> because if this Indexer is cloned all instances should
    // be stopping when the program is stopped, but then it adds a lot of boilerplate. So far, code
    // as I understand it doesn't clone `App` in a way it'd raise concern.
    pub indexing: bool,
    indexing_blocks: Arc<Mutex<HashMap<u64, bool>>>,
    // Add unique ID field, used for debugging and tracing
    id: u64,
}

// ------------------------------- DB Related ----------------------------------

impl Indexer {
    /// Index all previous blocks not yet indexed.
    fn index_previous_unindexed_blocks(&self, latest_block_height: u64) -> error::Result<()> {
        let last_indexed_block_height = self.handle.block_on(async {
            entity::blocks::Entity::find_last_block_height(&self.context.db).await
        })?;

        let last_indexed_block_height = match last_indexed_block_height {
            Some(height) => height as u64,
            None => 0, // happens when you index since genesis
        };

        let next_block_height = last_indexed_block_height + 1;
        if latest_block_height > next_block_height {
            return Ok(());
        }

        for block_height in next_block_height..=latest_block_height {
            let block_filename = self.indexer_path.block_path(block_height);

            let block_to_index = BlockToIndex::load_from_disk(block_filename.clone())
                .unwrap_or_else(|_err| {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_err, block_height, "Can't load block from disk");

                    panic!("can't load block from disk, can't continue as you'd be missing indexed blocks");
                });

            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height,
                indexer_id = self.id,
                "`index_previous_unindexed_blocks` started"
            );

            #[cfg(feature = "metrics")]
            metrics::counter!("indexer.previous_blocks.processed.total").increment(1);

            self.handle.block_on(async {
                block_to_index
                    .save(
                        self.context.db.clone(),
                        self.context.event_cache.clone(),
                        self.id,
                    )
                    .await?;

                Ok::<(), error::IndexerError>(())
            })?;

            if !self.keep_blocks {
                if let Err(_err) = BlockToIndex::delete_from_disk(block_filename.clone()) {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        error = %_err,
                        block_height,
                        block_filename = %block_filename.display(),
                        "Can't delete block from disk"
                    );
                }
            }

            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height,
                indexer_id = self.id,
                "`index_previous_unindexed_blocks` ended"
            );
        }

        Ok(())
    }

    pub fn save_block(
        db: DatabaseConnection,
        event_cache: EventCacheWriter,
        block: BlockAndBlockOutcomeWithHttpDetails,
    ) -> error::Result<()> {
        Ok(())
    }
}

impl IndexerTrait for Indexer {
    fn last_indexed_block_height(&self) -> grug_app::IndexerResult<Option<u64>> {
        let last_indexed_block_height = self
            .handle
            .block_on(async {
                entity::blocks::Entity::find_last_block_height(&self.context.db).await
            })
            .map_err(|e| grug_app::IndexerError::hook(e.to_string()))?;

        Ok(last_indexed_block_height.map(|h| h as u64))
    }

    fn start(&mut self, storage: &dyn Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "metrics")]
        crate::metrics::init_indexer_metrics();

        self.handle.block_on(async {
            self.context.migrate_db().await?;

            Ok::<(), error::IndexerError>(())
        })?;

        match LAST_FINALIZED_BLOCK.load(storage) {
            Err(_err) => {
                // This happens when the chain starts at genesis
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_err, "No `LAST_FINALIZED_BLOCK` found");
            },
            Ok(block) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    block_height = block.height,
                    "Start called, found a previous block"
                );

                self.index_previous_unindexed_blocks(block.height)?;
            },
        }

        self.indexing = true;

        Ok(())
    }

    fn shutdown(&mut self) -> grug_app::IndexerResult<()> {
        // Avoid running this twice when called manually and from `Drop`
        if !self.indexing {
            return Ok(());
        }

        self.indexing = false;

        // NOTE: This is to allow the indexer to commit all db transactions since this is done
        // async. It just loops quickly making sure no indexing blocks are remaining.
        for _ in 0..10 {
            if self
                .indexing_blocks
                .lock()
                .expect("can't lock indexing_blocks")
                .is_empty()
            {
                break;
            }

            sleep(Duration::from_millis(10));
        }

        #[cfg(feature = "tracing")]
        {
            let blocks = self
                .indexing_blocks
                .lock()
                .expect("can't lock indexing_blocks");
            if !blocks.is_empty() {
                tracing::warn!(
                    indexer_id = self.id,
                    "Some blocks are still being indexed. Maybe `non_blocking_indexer` `post_indexing` wasn't called by the main app?"
                );
            }
        }

        Ok(())
    }

    fn pre_indexing(
        &self,
        _block_height: u64,
        _ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        if !self.indexing {
            return Err(grug_app::IndexerError::not_running());
        }

        Ok(())
    }

    fn index_block(
        &self,
        _block: &Block,
        _block_outcome: &BlockOutcome,
        _ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        if !self.indexing {
            return Err(grug_app::IndexerError::not_running());
        }

        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        _querier: Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        if !self.indexing {
            return Err(grug_app::IndexerError::not_running());
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, "`post_indexing` called");

        self.indexing_blocks
            .lock()
            .map_err(|_| grug_app::IndexerError::mutex_poisoned())?
            .insert(block_height, true);

        let context = self.context.clone();
        // let block_to_index = self.find_or_fail(block_height)?;
        // let blocks = self.blocks.clone();
        // let keep_blocks = self.keep_blocks;
        // let block_filename = self
        //     .indexer_path
        //     .block_path(block_to_index.block.info.height);

        let id = self.id;

        // TODO: remove this once we extracted the caching to its own crate
        // ctx.insert(block_to_index.clone());

        // ctx.insert(context.pubsub.clone());
        // ctx.insert(block_to_index.block.clone());
        // ctx.insert(block_to_index.block_outcome.clone());

        let handle = self.handle.spawn(async move {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                block_height,
                indexer_id = id,
                "`post_indexing` async work started"
            );

            #[allow(clippy::map_identity)]
            if let Err(_err) =
                Self::save_block(context.db.clone(), context.event_cache.clone(), id).await
            {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    err = %_err,
                    indexer_id = id,
                    block_height,
                    "Can't save to db in `post_indexing`"
                );

                #[cfg(feature = "metrics")]
                metrics::counter!("indexer.errors.save.total").increment(1);

                return Ok(());
            }

            #[cfg(feature = "metrics")]
            metrics::counter!("indexer.blocks.processed.total").increment(1);

            if !keep_blocks {
                if let Err(_err) = BlockToIndex::delete_from_disk(block_filename.clone()) {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        error = %_err,
                        block_filename = %block_filename.display(),
                        "Can't delete block from disk in post_indexing"
                    );

                    return Ok(());
                }

                #[cfg(feature = "metrics")]
                metrics::counter!("indexer.blocks.deleted.total").increment(1);
            } else {
                // compress takes CPU, so we do it in a spawned blocking task
                if let Err(_err) = tokio::task::spawn_blocking(move || {
                    if let Err(_err) = BlockToIndex::compress_file(block_filename.clone()) {
                        #[cfg(feature = "tracing")]
                        tracing::error!(
                            error = %_err,
                            block_filename = %block_filename.display(),
                            "Can't compress block on disk in post_indexing"
                        );
                    }
                })
                .await
                {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_err, "`spawn_blocking` error compressing block file");
                }

                #[cfg(feature = "metrics")]
                metrics::counter!("indexer.blocks.compressed.total").increment(1);
            }

            if let Err(_err) = context.pubsub.publish(block_height).await {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    err = %_err,
                    indexer_id = id,
                    block_height,
                    "Can't publish block minted in `post_indexing`"
                );

                #[cfg(feature = "metrics")]
                metrics::counter!("indexer.errors.pubsub.total").increment(1);

                return Ok(());
            }

            #[cfg(feature = "metrics")]
            metrics::counter!("indexer.pubsub.published.total").increment(1);

            #[cfg(feature = "tracing")]
            tracing::info!(block_height, indexer_id = id, "`post_indexing` finished");

            Ok::<(), grug_app::IndexerError>(())
        });

        self.handle
            .block_on(handle)
            .map_err(|e| grug_app::IndexerError::hook(e.to_string()))??;

        Ok(())
    }

    /// Wait for all blocks to be indexed
    fn wait_for_finish(&self) -> grug_app::IndexerResult<()> {
        for _ in 0..100 {
            if self
                .indexing_blocks
                .lock()
                .map_err(|_| grug_app::IndexerError::mutex_poisoned())?
                .is_empty()
            {
                break;
            }

            sleep(Duration::from_millis(100));
        }

        let blocks = self
            .indexing_blocks
            .lock()
            .expect("can't lock indexing_blocks");
        if !blocks.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Indexer `wait_for_finish` ended, still has {} blocks: {:?}",
                blocks.len(),
                blocks.keys()
            );
        }

        Ok(())
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        // If the DatabaseTransactions are left open (not committed) its `Drop` implementation
        // expects a Tokio context. We must call `commit` manually on it within our Tokio
        // context.
        self.shutdown().expect("can't shutdown indexer");
    }
}

// ------------------------------- RuntimeHandler ------------------------------

/// Wrapper around Tokio runtime to allow running in sync context
#[derive(Debug)]
pub struct RuntimeHandler {
    pub runtime: Option<Runtime>,
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

// Note: Removed Deref implementation to allow proper referencing
// Access the handle via .handle() method instead

impl RuntimeHandler {
    /// Create a RuntimeHandler from an existing tokio Handle
    /// This shares the same runtime as the original handle
    pub fn from_handle(handle: Handle) -> Self {
        Self {
            runtime: None, // No ownership of runtime, just using existing one
            handle,
        }
    }

    /// Get a reference to the tokio Handle
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Spawn a task on the runtime
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.handle.spawn(future)
    }

    /// Runs a future in the Tokio runtime, blocking the current thread until the future is resolved.
    ///
    /// This function allows running in a sync context by using the appropriate method
    /// based on whether we own the runtime or are using an existing one.
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
            // Check if we're in an actix-web worker thread context
            if let Some(name) = std::thread::current().name() {
                if name.contains("actix-") {
                    // For actix-web worker threads, use futures::executor::block_on
                    // which doesn't require multi-threaded runtime
                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        "Using futures::executor::block_on for actix-web worker thread: {}",
                        name
                    );

                    let result = futures::executor::block_on(closure);

                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        "futures::executor::block_on completed for actix-web worker thread: {}",
                        name
                    );

                    return result;
                }
            }

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
        let mut indexer: Indexer = IndexerBuilder::default().with_memory_database().build()?;
        let storage = MockStorage::new();

        assert!(!indexer.indexing);
        indexer.start(&storage).expect("can't start indexer");
        assert!(indexer.indexing);

        indexer.shutdown().expect("can't shutdown Indexer");
        assert!(!indexer.indexing);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn build_without_hooks() -> anyhow::Result<()> {
        let mut indexer: Indexer = IndexerBuilder::default().with_memory_database().build()?;
        let storage = MockStorage::new();

        assert!(!indexer.indexing);
        indexer.start(&storage).expect("can't start Indexer");
        assert!(indexer.indexing);

        indexer.shutdown().expect("can't shutdown Indexer");
        assert!(!indexer.indexing);

        Ok(())
    }
}
