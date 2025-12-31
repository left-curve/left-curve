use {
    async_trait::async_trait,
    futures::executor::block_on,
    grug_app::{APP_CONFIG, CONFIG, Indexer, IndexerResult, LAST_FINALIZED_BLOCK},
    grug_types::{Config, Json},
    std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    },
    tokio::{
        sync::{Mutex, RwLock},
        time::sleep,
    },
};

// Re-export for convenience
pub use grug_app::IndexerError;

/// A composable indexer that can own multiple indexers and coordinate between them
#[derive(Clone)]
pub struct HookedIndexer {
    /// List of registered indexers
    indexers: Arc<RwLock<Vec<Box<dyn Indexer + Send + Sync>>>>,
    /// Whether the indexer is currently running
    is_running: Arc<AtomicBool>,
    post_indexing_tasks: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
}

impl HookedIndexer {
    pub fn new() -> Self {
        Self {
            indexers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            post_indexing_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add an indexer to the composition
    pub async fn add_indexer<I>(&mut self, indexer: I) -> Result<&mut Self, grug_app::IndexerError>
    where
        I: Indexer + Send + Sync + 'static,
    {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(grug_app::IndexerError::already_running());
        }

        self.indexers.write().await.push(Box::new(indexer));
        Ok(self)
    }

    /// Check if the indexer is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    // Synchronous version for tests and sync contexts
    pub fn indexer_count_sync(&self) -> usize {
        // For sync access, use futures::executor::block_on
        block_on(async { self.indexer_count().await })
    }

    /// Get the number of registered indexers
    pub async fn indexer_count(&self) -> usize {
        self.indexers.read().await.len()
    }

    /// This will reindex all indexers from their last indexed block height
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn reindex(&self, storage: &dyn grug_types::Storage) -> IndexerResult<()> {
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
                    "Start called, found LAST_FINALIZED_BLOCK"
                );

                let mut indexers = self.indexers.write().await;

                let cfg = CONFIG.load(storage).map_err(|e| {
                    grug_app::IndexerError::storage(format!("Failed to load CONFIG: {e}"))
                })?;

                let app_cfg = APP_CONFIG.load(storage).map_err(|e| {
                    grug_app::IndexerError::storage(format!("Failed to load APP_CONFIG: {e}"))
                })?;

                // 1. We get the lowest last indexed block height among all indexers,
                let mut min_heights = Vec::new();
                for indexer in indexers.iter_mut() {
                    min_heights.push(indexer.last_indexed_block_height().await.ok().flatten());
                }

                let min_height = min_heights
                    .iter()
                    .flatten()
                    .min()
                    .cloned()
                    .unwrap_or_default();

                if min_height >= block.height {
                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        block_height = block.height,
                        "No reindexing needed, all indexers are up to date",
                    );
                    return Ok(());
                }

                let mut errors = Vec::new();

                // 2. We run all indexers to reindex until the last finalized block, like it would
                // have happened during normal operation but calling a different method.
                for block_height in (min_height + 1)..=block.height {
                    let mut ctx = grug_app::IndexerContext::new();
                    for indexer in &mut indexers.iter_mut() {
                        if let Err(err) = indexer.pre_indexing(block_height, &mut ctx).await {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Error in start calling reindex: {:?}", err);
                            errors.push(err.to_string());
                        }
                    }

                    // NOTE: I skip `index_block` here because we don't have the actual block data and
                    // it's only used to store data on disk, which we already have.
                    // In the future this can be an issue if an indexer relies on `index_block`

                    // I recreate a context like classic indexing code path
                    let mut ctx = grug_app::IndexerContext::new();
                    for indexer in &mut indexers.iter_mut() {
                        if let Err(err) = indexer
                            .post_indexing(block_height, cfg.clone(), app_cfg.clone(), &mut ctx)
                            .await
                        {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Error in start calling reindex: {:?}", err);
                            errors.push(err.to_string());
                        }
                    }

                    if !errors.is_empty() {
                        return Err(grug_app::IndexerError::multiple(errors));
                    }
                }
            },
        }

        Ok(())
    }
}

impl Default for HookedIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Indexer for HookedIndexer {
    /// Start all indexers
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn start(&mut self, storage: &dyn grug_types::Storage) -> IndexerResult<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(grug_app::IndexerError::already_running());
        }

        #[cfg(feature = "tracing")]
        {
            let count = self.indexer_count().await;
            tracing::debug!("Starting HookedIndexer with {} indexers", count);
        }

        let mut errors = Vec::new();

        // Call start on all indexers, running required migrations
        // With tokio::sync::RwLock, we can hold the lock across await points
        let mut guard = self.indexers.write().await;
        for indexer in guard.iter_mut() {
            if let Err(err) = indexer.start(storage).await {
                #[cfg(feature = "tracing")]
                tracing::error!("Error in start: {:?}", err);
                errors.push(err.to_string());
            }
        }
        drop(guard);

        self.reindex(storage).await?;

        self.is_running.store(true, Ordering::Relaxed);

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::multiple(errors));
        }

        Ok(())
    }

    /// Shutdown all indexers in reverse order
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn shutdown(&mut self) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(()); // Already shut down
        }

        self.wait_for_finish().await?;

        // Call shutdown on all indexers in reverse order
        let mut errors = Vec::new();
        let mut guard = self.indexers.write().await;
        for indexer in guard.iter_mut().rev() {
            if let Err(err) = indexer.shutdown().await {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %err, indexer_name = indexer.name(), "Error in shutdown");

                errors.push(err.to_string());
            }
        }
        drop(guard);

        self.is_running.store(false, Ordering::Relaxed);

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::multiple(errors));
        }

        Ok(())
    }

    /// Run pre_indexing for each block height
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn pre_indexing(
        &self,
        block_height: u64,
        ctx: &mut grug_app::IndexerContext,
    ) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(grug_app::IndexerError::not_running());
        }

        let mut errors = Vec::new();

        let guard = self.indexers.read().await;
        for indexer in guard.iter() {
            if let Err(err) = indexer.pre_indexing(block_height, ctx).await {
                #[cfg(feature = "tracing")]
                tracing::error!("Error in pre_indexing: {:?}", err);

                errors.push(err.to_string());
            }
        }
        drop(guard);

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::multiple(errors));
        }

        Ok(())
    }

    /// Index a block by calling all registered indexers
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
        ctx: &mut grug_app::IndexerContext,
    ) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(grug_app::IndexerError::not_running());
        }

        let mut errors = Vec::new();
        let guard = self.indexers.read().await;
        for indexer in guard.iter() {
            if let Err(err) = indexer.index_block(block, block_outcome, ctx).await {
                #[cfg(feature = "tracing")]
                tracing::error!("Error in index_block: {:?}", err);
                errors.push(err.to_string());
            }
        }
        drop(guard);

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::multiple(errors));
        }

        Ok(())
    }

    /// Run post_indexing in a separate task for each block height
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn post_indexing(
        &self,
        block_height: u64,
        cfg: Config,
        app_cfg: Json,
        ctx: &mut grug_app::IndexerContext,
    ) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(grug_app::IndexerError::not_running());
        }

        let post_indexing_tasks = self.post_indexing_tasks.clone();
        let indexers = self.indexers.clone();

        // Clone the `IndexerContext` to avoid borrowing issues.
        // I do this clone because:
        // 1. `IndexerContext` isn't used in the main task after `post_indexing` is called
        // 2. `post_indexing` is called in a separate task
        let mut ctx = ctx.clone();

        let handle = tokio::spawn(async move {
            let mut errors = Vec::new();

            let indexers_guard = indexers.read().await;

            for indexer in indexers_guard.iter() {
                if let Err(err) = indexer
                    .post_indexing(block_height, cfg.clone(), app_cfg.clone(), &mut ctx)
                    .await
                {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        indexer = indexer.name(),
                        err = %err,
                        block_height,
                        "Error post_indexing"
                    );

                    errors.push(err.to_string());
                }
            }

            drop(indexers_guard);

            // Remove this task from the map when it completes
            let mut tasks_guard = post_indexing_tasks.lock().await;
            tasks_guard.remove(&block_height);
            drop(tasks_guard);

            if !errors.is_empty() {
                #[cfg(feature = "tracing")]
                tracing::error!("Errors in post_indexing: {:?}", errors);
            }
        });

        // Store the task handle with its block height
        self.post_indexing_tasks
            .lock()
            .await
            .insert(block_height, handle);

        Ok(())
    }

    /// Wait for all indexers and post_indexing tasks to finish
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn wait_for_finish(&self) -> IndexerResult<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Waiting for indexer to finish");

        // 1. We have our own internal tasks that are running post_indexing
        loop {
            let tasks_guard = self.post_indexing_tasks.lock().await;

            if tasks_guard.is_empty() {
                break;
            }

            // Collect block heights for logging
            let block_heights: Vec<u64> = tasks_guard.keys().copied().collect();
            let task_count = tasks_guard.len();
            drop(tasks_guard);

            #[cfg(feature = "tracing")]
            tracing::debug!(
                tasks = task_count,
                blocks = ?block_heights,
                "Waiting for post_indexing tasks to finish"
            );

            // Wait a bit and check which tasks have completed
            sleep(Duration::from_millis(100)).await;

            // Remove completed tasks
            let mut tasks_guard = self.post_indexing_tasks.lock().await;
            tasks_guard.retain(|&block_height, handle| {
                if handle.is_finished() {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(block_height, "Post_indexing task completed");
                    false
                } else {
                    true
                }
            });
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("Waiting for indexers to finish");

        // 2. We have the indexers that are potentially running their own way
        let guard = self.indexers.read().await;
        for indexer in guard.iter() {
            indexer.wait_for_finish().await?;
        }
        drop(guard);

        #[cfg(feature = "tracing")]
        tracing::debug!("Waited for indexers to finish");

        Ok(())
    }
}

impl Drop for HookedIndexer {
    fn drop(&mut self) {
        // Since shutdown is now async, we can't call it from Drop
        // Just mark as not running - the actual cleanup will happen when the async context completes
        self.is_running.store(false, Ordering::Relaxed);
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::MockStorage};

    #[derive(Default)]
    struct TestIndexer {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl TestIndexer {
        fn record_call(&self, method: &str) {
            self.calls.lock().unwrap().push(method.to_string());
        }
    }

    #[async_trait]
    impl Indexer for TestIndexer {
        async fn start(&mut self, _storage: &dyn grug_types::Storage) -> IndexerResult<()> {
            self.record_call("start");
            Ok(())
        }

        async fn shutdown(&mut self) -> IndexerResult<()> {
            self.record_call("shutdown");
            Ok(())
        }

        async fn pre_indexing(
            &self,
            _block_height: u64,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            self.record_call("pre_indexing");
            Ok(())
        }

        async fn index_block(
            &self,
            _block: &grug_types::Block,
            _block_outcome: &grug_types::BlockOutcome,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            self.record_call("index_block");
            Ok(())
        }

        async fn post_indexing(
            &self,
            _block_height: u64,
            _cfg: Config,
            _app_cfg: Json,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            self.record_call("post_indexing");
            Ok(())
        }

        async fn wait_for_finish(&self) -> IndexerResult<()> {
            self.record_call("wait_for_finish");
            Ok(())
        }
    }

    #[test]
    fn test_hooked_indexer_creation() {
        let indexer = HookedIndexer::new();
        assert_eq!(indexer.indexer_count_sync(), 0);
        assert!(!indexer.is_running());
    }

    #[tokio::test]
    async fn test_add_indexers() {
        let mut hooked_indexer = HookedIndexer::new();

        hooked_indexer
            .add_indexer(TestIndexer::default())
            .await
            .unwrap()
            .add_indexer(TestIndexer::default())
            .await
            .unwrap();

        assert_eq!(hooked_indexer.indexer_count().await, 2);
    }

    #[tokio::test]
    async fn test_start_and_shutdown() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer
            .add_indexer(TestIndexer::default())
            .await
            .unwrap();

        let storage = MockStorage::new();

        // Test start
        assert!(!hooked_indexer.is_running());
        hooked_indexer.start(&storage).await.unwrap();
        assert!(hooked_indexer.is_running());

        // Test shutdown
        hooked_indexer.shutdown().await.unwrap();
        assert!(!hooked_indexer.is_running());
    }

    #[tokio::test]
    async fn test_double_start_fails() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer
            .add_indexer(TestIndexer::default())
            .await
            .unwrap();

        let storage = MockStorage::new();

        hooked_indexer.start(&storage).await.unwrap();

        // Second start should fail
        assert!(hooked_indexer.start(&storage).await.is_err());
    }

    #[tokio::test]
    async fn test_operations_when_not_running() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer
            .add_indexer(TestIndexer::default())
            .await
            .unwrap();

        let mut ctx = grug_app::IndexerContext::new();

        // Operations should fail when not running
        assert!(hooked_indexer.pre_indexing(1, &mut ctx).await.is_err());

        let block = grug_types::Block {
            info: grug_types::BlockInfo {
                height: 1,
                timestamp: grug_types::Timestamp::from_seconds(1),
                hash: [0u8; 32].into(),
            },
            txs: vec![],
        };

        let outcome = grug_types::BlockOutcome {
            height: 1,
            app_hash: grug_types::Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        assert!(
            hooked_indexer
                .index_block(&block, &outcome, &mut ctx)
                .await
                .is_err()
        );
    }

    /// Example indexer that stores data in the context for later indexers to use
    #[derive(Default)]
    struct DataProducerIndexer {
        id: String,
    }

    impl DataProducerIndexer {
        fn new(id: &str) -> Self {
            Self { id: id.to_string() }
        }
    }

    #[async_trait]
    impl Indexer for DataProducerIndexer {
        async fn pre_indexing(
            &self,
            block_height: u64,
            ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            // Store some data that other indexers can use
            ctx.insert(format!("data_from_{}_at_height_{}", self.id, block_height));
            Ok(())
        }
    }

    /// Example indexer that consumes data from the context
    #[derive(Default)]
    struct DataConsumerIndexer {
        consumed_data: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl DataConsumerIndexer {
        fn new() -> Self {
            Self {
                consumed_data: Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl Indexer for DataConsumerIndexer {
        async fn index_block(
            &self,
            _block: &grug_types::Block,
            _block_outcome: &grug_types::BlockOutcome,
            ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            // Try to consume data stored by other indexers
            if let Some(data) = ctx.get::<String>() {
                self.consumed_data.lock().unwrap().push(data.clone());
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_context_data_passing() {
        let mut hooked_indexer = HookedIndexer::new();

        // Add a producer indexer that stores data
        let producer = DataProducerIndexer::new("producer1");
        hooked_indexer.add_indexer(producer).await.unwrap();

        // Add a consumer indexer that reads data
        let consumer = DataConsumerIndexer::new();
        let consumer_data = consumer.consumed_data.clone();
        hooked_indexer.add_indexer(consumer).await.unwrap();

        let storage = MockStorage::new();
        hooked_indexer.start(&storage).await.unwrap();

        let block = grug_types::Block {
            info: grug_types::BlockInfo {
                height: 42,
                timestamp: grug_types::Timestamp::from_seconds(123456789),
                hash: grug_types::Hash256::ZERO,
            },
            txs: vec![],
        };
        let outcome = grug_types::BlockOutcome {
            height: 42,
            app_hash: grug_types::Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        let mut ctx = grug_app::IndexerContext::new();

        // Run the indexing pipeline
        hooked_indexer.pre_indexing(42, &mut ctx).await.unwrap();
        hooked_indexer
            .index_block(&block, &outcome, &mut ctx)
            .await
            .unwrap();

        // Verify that data was passed from producer to consumer
        {
            let consumed_data = consumer_data.lock().unwrap();
            assert_eq!(consumed_data.len(), 1);
            assert_eq!(consumed_data[0], "data_from_producer1_at_height_42");
        }

        hooked_indexer.shutdown().await.unwrap();
    }
}
