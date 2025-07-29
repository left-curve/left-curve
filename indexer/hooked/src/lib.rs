use {
    grug_app::{Indexer, IndexerResult},
    std::{
        collections::HashMap,
        sync::{Arc, Mutex, RwLock},
        thread::sleep,
        time::Duration,
    },
};

// Re-export for convenience
pub use grug_app::IndexerError;

/// A composable indexer that can own multiple indexers and coordinate between them
pub struct HookedIndexer {
    /// List of registered indexers
    indexers: Arc<RwLock<Vec<Box<dyn Indexer + Send + Sync>>>>,
    /// Whether the indexer is currently running
    is_running: bool,
    post_indexing_threads: Arc<Mutex<HashMap<u64, bool>>>,
}

impl HookedIndexer {
    pub fn new() -> Self {
        Self {
            indexers: Arc::new(RwLock::new(Vec::new())),
            is_running: false,
            post_indexing_threads: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add an indexer to the composition
    pub fn add_indexer<I>(&mut self, indexer: I) -> Result<&mut Self, grug_app::IndexerError>
    where
        I: Indexer + Send + Sync + 'static,
    {
        if self.is_running {
            return Err(grug_app::IndexerError::AlreadyRunning);
        }

        self.indexers
            .write()
            .map_err(|_| grug_app::IndexerError::RwlockPoisoned)?
            .push(Box::new(indexer));
        Ok(self)
    }

    /// Check if the indexer is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the number of registered indexers
    pub fn indexer_count(&self) -> usize {
        self.indexers
            .read()
            .map(|indexers| indexers.len())
            .unwrap_or(0)
    }
}

impl Default for HookedIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexer for HookedIndexer {
    fn start(&mut self, storage: &dyn grug_types::Storage) -> IndexerResult<()> {
        if self.is_running {
            return Err(grug_app::IndexerError::AlreadyRunning);
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Starting HookedIndexer with {} indexers",
            self.indexer_count()
        );

        let mut errors = Vec::new();

        // Call start on all indexers
        for indexer in &mut self
            .indexers
            .write()
            .map_err(|_| grug_app::IndexerError::RwlockPoisoned)?
            .iter_mut()
        {
            if let Err(err) = indexer.start(storage) {
                #[cfg(feature = "tracing")]
                tracing::error!("Error in start: {:?}", err);
                errors.push(err.to_string());
            }
        }

        self.is_running = true;

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::Multiple(errors));
        }

        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        if !self.is_running {
            return Ok(()); // Already shut down
        }

        // Call shutdown on all indexers in reverse order
        let mut errors = Vec::new();
        for indexer in self
            .indexers
            .write()
            .map_err(|_| grug_app::IndexerError::RwlockPoisoned)?
            .iter_mut()
            .rev()
        {
            if let Err(err) = indexer.shutdown() {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %err, indexer_name = indexer.name(), "Error in shutdown");

                errors.push(err.to_string());
            }
        }

        self.is_running = false;

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::Multiple(errors));
        }

        Ok(())
    }

    fn pre_indexing(
        &self,
        block_height: u64,
        ctx: &mut grug_app::IndexerContext,
    ) -> IndexerResult<()> {
        if !self.is_running {
            return Err(grug_app::IndexerError::NotRunning);
        }

        let mut errors = Vec::new();

        for indexer in self
            .indexers
            .read()
            .map_err(|_| grug_app::IndexerError::RwlockPoisoned)?
            .iter()
        {
            if let Err(err) = indexer.pre_indexing(block_height, ctx) {
                #[cfg(feature = "tracing")]
                tracing::error!("Error in pre_indexing: {:?}", err);

                errors.push(err.to_string());
            }
        }

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::Multiple(errors));
        }

        Ok(())
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
        ctx: &mut grug_app::IndexerContext,
    ) -> IndexerResult<()> {
        if !self.is_running {
            return Err(grug_app::IndexerError::NotRunning);
        }

        let mut errors = Vec::new();
        for indexer in self
            .indexers
            .read()
            .map_err(|_| grug_app::IndexerError::RwlockPoisoned)?
            .iter()
        {
            if let Err(err) = indexer.index_block(block, block_outcome, ctx) {
                #[cfg(feature = "tracing")]
                tracing::error!("Error in index_block: {:?}", err);
                errors.push(err.to_string());
            }
        }

        if !errors.is_empty() {
            return Err(grug_app::IndexerError::Multiple(errors));
        }

        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> IndexerResult<()> {
        if !self.is_running {
            return Err(grug_app::IndexerError::NotRunning);
        }

        let post_indexing_threads = self.post_indexing_threads.clone();

        let indexers = self.indexers.clone();

        // Clone the `IndexerContext` to avoid borrowing issues.
        // I do this clone because:
        // 1. `IndexerContext` isn't used in the main thread after `post_indexing` is called
        // 2. `post_indexing` is called in a separate thread
        let mut ctx = ctx.clone();

        self.post_indexing_threads
            .lock()?
            .insert(block_height, true);

        std::thread::spawn(move || {
            let mut errors = Vec::new();

            for indexer in indexers
                .read()
                .map_err(|_| {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Rwlock poisoned in post_indexing");
                    grug_app::IndexerError::RwlockPoisoned
                })?
                .iter()
            {
                if let Err(err) = indexer.post_indexing(block_height, querier.clone(), &mut ctx) {
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

            post_indexing_threads
                .lock()
                .map_err(|_| {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Mutex poisoned in post_indexing");
                    grug_app::IndexerError::MutexPoisoned
                })?
                .remove(&block_height);

            if !errors.is_empty() {
                return Err(grug_app::IndexerError::Multiple(errors));
            }

            Ok::<(), IndexerError>(())
        });

        Ok(())
    }

    fn wait_for_finish(&self) -> IndexerResult<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Waiting for indexer to finish");

        // 1. We have our own internal threads that are running post_indexing
        for _ in 0..100 {
            let post_indexing_threads = self
                .post_indexing_threads
                .lock()
                .map_err(|_| grug_app::IndexerError::MutexPoisoned)?
                .len();

            #[cfg(feature = "tracing")]
            tracing::debug!(
                threads = post_indexing_threads,
                "Waiting for threads to finish",
            );

            if post_indexing_threads == 0 {
                break;
            }

            sleep(Duration::from_millis(100));
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("Waiting for indexers to finish");

        // 2. We have the indexers that are potentially running their own way
        for indexer in self
            .indexers
            .read()
            .map_err(|_| grug_app::IndexerError::RwlockPoisoned)?
            .iter()
        {
            indexer.wait_for_finish()?;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("Waited for indexers to finish");

        Ok(())
    }
}

impl Drop for HookedIndexer {
    fn drop(&mut self) {
        self.shutdown().expect("can't shutdown hooked_indexer");
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

    impl Indexer for TestIndexer {
        fn start(&mut self, _storage: &dyn grug_types::Storage) -> IndexerResult<()> {
            self.record_call("start");
            Ok(())
        }

        fn shutdown(&mut self) -> IndexerResult<()> {
            self.record_call("shutdown");
            Ok(())
        }

        fn pre_indexing(
            &self,
            _block_height: u64,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            self.record_call("pre_indexing");
            Ok(())
        }

        fn index_block(
            &self,
            _block: &grug_types::Block,
            _block_outcome: &grug_types::BlockOutcome,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            self.record_call("index_block");
            Ok(())
        }

        fn post_indexing(
            &self,
            _block_height: u64,
            _querier: Arc<dyn grug_app::QuerierProvider>,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            self.record_call("post_indexing");
            Ok(())
        }

        fn wait_for_finish(&self) -> IndexerResult<()> {
            self.record_call("wait_for_finish");
            Ok(())
        }
    }

    #[test]
    fn test_hooked_indexer_creation() {
        let indexer = HookedIndexer::new();
        assert_eq!(indexer.indexer_count(), 0);
        assert!(!indexer.is_running());
    }

    #[test]
    fn test_add_indexers() {
        let mut hooked_indexer = HookedIndexer::new();

        hooked_indexer
            .add_indexer(TestIndexer::default())
            .unwrap()
            .add_indexer(TestIndexer::default())
            .unwrap();

        assert_eq!(hooked_indexer.indexer_count(), 2);
    }

    #[test]
    fn test_start_and_shutdown() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer.add_indexer(TestIndexer::default()).unwrap();

        let storage = MockStorage::new();

        // Test start
        assert!(!hooked_indexer.is_running());
        hooked_indexer.start(&storage).unwrap();
        assert!(hooked_indexer.is_running());

        // Test shutdown
        hooked_indexer.shutdown().unwrap();
        assert!(!hooked_indexer.is_running());
    }

    #[test]
    fn test_double_start_fails() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer.add_indexer(TestIndexer::default()).unwrap();

        let storage = MockStorage::new();

        hooked_indexer.start(&storage).unwrap();

        // Second start should fail
        assert!(hooked_indexer.start(&storage).is_err());
    }

    #[test]
    fn test_operations_when_not_running() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer.add_indexer(TestIndexer::default()).unwrap();

        let mut ctx = grug_app::IndexerContext::new();

        // Operations should fail when not running
        assert!(hooked_indexer.pre_indexing(1, &mut ctx).is_err());

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

    impl Indexer for DataProducerIndexer {
        fn start(&mut self, _storage: &dyn grug_types::Storage) -> IndexerResult<()> {
            Ok(())
        }

        fn shutdown(&mut self) -> IndexerResult<()> {
            Ok(())
        }

        fn pre_indexing(
            &self,
            block_height: u64,
            ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            // Store some data that other indexers can use
            ctx.insert(format!("data_from_{}_at_height_{}", self.id, block_height));
            Ok(())
        }

        fn index_block(
            &self,
            _block: &grug_types::Block,
            _block_outcome: &grug_types::BlockOutcome,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            Ok(())
        }

        fn post_indexing(
            &self,
            _block_height: u64,
            _querier: Arc<dyn grug_app::QuerierProvider>,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            Ok(())
        }

        fn wait_for_finish(&self) -> IndexerResult<()> {
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

    impl Indexer for DataConsumerIndexer {
        fn start(&mut self, _storage: &dyn grug_types::Storage) -> IndexerResult<()> {
            Ok(())
        }

        fn shutdown(&mut self) -> IndexerResult<()> {
            Ok(())
        }

        fn pre_indexing(
            &self,
            _block_height: u64,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            Ok(())
        }

        fn index_block(
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

        fn post_indexing(
            &self,
            _block_height: u64,
            _querier: Arc<dyn grug_app::QuerierProvider>,
            _ctx: &mut grug_app::IndexerContext,
        ) -> IndexerResult<()> {
            Ok(())
        }

        fn wait_for_finish(&self) -> IndexerResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_context_data_passing() {
        let mut hooked_indexer = HookedIndexer::new();

        // Add a producer indexer that stores data
        let producer = DataProducerIndexer::new("producer1");
        hooked_indexer.add_indexer(producer).unwrap();

        // Add a consumer indexer that reads data
        let consumer = DataConsumerIndexer::new();
        let consumer_data = consumer.consumed_data.clone();
        hooked_indexer.add_indexer(consumer).unwrap();

        let storage = MockStorage::new();
        hooked_indexer.start(&storage).unwrap();

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
        hooked_indexer.pre_indexing(42, &mut ctx).unwrap();
        hooked_indexer
            .index_block(&block, &outcome, &mut ctx)
            .unwrap();

        // Verify that data was passed from producer to consumer
        let consumed_data = consumer_data.lock().unwrap();
        assert_eq!(consumed_data.len(), 1);
        assert_eq!(consumed_data[0], "data_from_producer1_at_height_42");

        hooked_indexer.shutdown().unwrap();
    }
}
