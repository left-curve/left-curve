use {grug_app::Indexer, grug_types::Storage};

// Re-export modules for easier access
pub mod context;
pub mod error;

pub use {
    context::IndexerContext,
    error::{HookedIndexerError, Result},
};

/// Simple adapter that wraps an indexer with error conversion
struct SimpleIndexerAdapter<I> {
    indexer: std::sync::Mutex<I>,
}

impl<I> SimpleIndexerAdapter<I> {
    fn new(indexer: I) -> Self {
        Self {
            indexer: std::sync::Mutex::new(indexer),
        }
    }
}

unsafe impl<I> Send for SimpleIndexerAdapter<I> where I: Send {}
unsafe impl<I> Sync for SimpleIndexerAdapter<I> where I: Send + Sync {}

impl<I> Indexer for SimpleIndexerAdapter<I>
where
    I: Indexer + Send + Sync,
    I::Error: Into<HookedIndexerError>,
{
    type Error = HookedIndexerError;

    fn start(&mut self, storage: &dyn Storage) -> Result<()> {
        let mut indexer = self.indexer.lock().unwrap();
        indexer.start(storage).map_err(|e| e.into())
    }

    fn shutdown(&mut self) -> Result<()> {
        let mut indexer = self.indexer.lock().unwrap();
        indexer.shutdown().map_err(|e| e.into())
    }

    fn pre_indexing(&self, block_height: u64) -> Result<()> {
        let indexer = self.indexer.lock().unwrap();
        indexer.pre_indexing(block_height).map_err(|e| e.into())
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
    ) -> Result<()> {
        let indexer = self.indexer.lock().unwrap();
        indexer
            .index_block(block, block_outcome)
            .map_err(|e| e.into())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: Box<dyn grug_app::QuerierProvider>,
    ) -> Result<()> {
        let indexer = self.indexer.lock().unwrap();
        indexer
            .post_indexing(block_height, querier)
            .map_err(|e| e.into())
    }

    fn wait_for_finish(&self) {
        let indexer = self.indexer.lock().unwrap();
        indexer.wait_for_finish();
    }
}

/// A composable indexer that can own multiple indexers and coordinate between them
pub struct HookedIndexer {
    /// List of registered indexers with error conversion
    indexers: Vec<Box<dyn Indexer<Error = HookedIndexerError> + Send + Sync>>,
    /// Shared context that gets passed to middleware (not directly to indexers)
    context: IndexerContext,
    /// Whether the indexer is currently running
    is_running: bool,
}

impl HookedIndexer {
    pub fn new() -> Self {
        Self {
            indexers: Vec::new(),
            context: IndexerContext::new(),
            is_running: false,
        }
    }

    /// Add an indexer to the composition
    pub fn add_indexer<I>(&mut self, indexer: I) -> &mut Self
    where
        I: Indexer + Send + Sync + 'static,
        I::Error: Into<HookedIndexerError>,
    {
        let adapter = SimpleIndexerAdapter::new(indexer);
        self.indexers.push(Box::new(adapter));
        self
    }

    /// Get a reference to the context
    pub fn context(&self) -> &IndexerContext {
        &self.context
    }

    /// Get a mutable reference to the context
    pub fn context_mut(&mut self) -> &mut IndexerContext {
        &mut self.context
    }

    /// Check if the indexer is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get the number of registered indexers
    pub fn indexer_count(&self) -> usize {
        self.indexers.len()
    }
}

impl Default for HookedIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexer for HookedIndexer {
    type Error = HookedIndexerError;

    fn start(&mut self, storage: &dyn Storage) -> Result<()> {
        if self.is_running {
            return Err(HookedIndexerError::AlreadyRunning);
        }

        // Initialize context
        self.context
            .set_property("indexer_started".to_string(), "true".to_string());

        // Call start on all indexers
        for indexer in &mut self.indexers {
            indexer.start(storage)?;
        }

        self.is_running = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(()); // Already shut down
        }

        // Call shutdown on all indexers in reverse order
        let mut errors = Vec::new();
        for indexer in self.indexers.iter_mut().rev() {
            if let Err(e) = indexer.shutdown() {
                errors.push(e.to_string());
            }
        }

        self.is_running = false;

        if !errors.is_empty() {
            return Err(HookedIndexerError::Multiple(errors));
        }

        Ok(())
    }

    fn pre_indexing(&self, block_height: u64) -> Result<()> {
        if !self.is_running {
            return Err(HookedIndexerError::NotRunning);
        }

        for indexer in &self.indexers {
            indexer.pre_indexing(block_height)?;
        }

        Ok(())
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
    ) -> Result<()> {
        if !self.is_running {
            return Err(HookedIndexerError::NotRunning);
        }

        for indexer in &self.indexers {
            indexer.index_block(block, block_outcome)?;
        }

        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: Box<dyn grug_app::QuerierProvider>,
    ) -> Result<()> {
        if !self.is_running {
            return Err(HookedIndexerError::NotRunning);
        }

        // We need to clone the querier for each indexer since they each take ownership
        // This is a limitation when composing indexers that expect owned QuerierProvider
        for (i, indexer) in self.indexers.iter().enumerate() {
            // For the last indexer, we can pass the original querier
            // For others, we need to create a simple no-op wrapper since proper cloning
            // is complex and the current QuerierProvider doesn't support it
            if i == self.indexers.len() - 1 {
                indexer.post_indexing(block_height, querier)?;
                break;
            } else {
                // Create a no-op querier for intermediate indexers
                // TODO: This is a limitation that should be addressed in the Indexer trait
                indexer.post_indexing(block_height, Box::new(NoOpQuerierProvider))?;
            }
        }

        Ok(())
    }

    fn wait_for_finish(&self) {
        for indexer in self.indexers.iter().rev() {
            indexer.wait_for_finish();
        }
    }
}

/// Simple no-op QuerierProvider for intermediate indexers in post_indexing
/// This is a workaround for the ownership limitation in post_indexing
struct NoOpQuerierProvider;

impl grug_app::QuerierProvider for NoOpQuerierProvider {
    fn do_query_chain(
        &self,
        _req: grug_types::Query,
        _query_depth: usize,
    ) -> grug_types::GenericResult<grug_types::QueryResponse> {
        // This is a limitation - intermediate indexers in the chain can't use the querier
        Err("QuerierProvider not available for intermediate indexers".into())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_app::{Indexer, QuerierProvider},
        grug_types::{Block, BlockOutcome, MockStorage, Storage},
        std::{
            convert::Infallible,
            sync::{Arc, RwLock},
        },
    };

    #[derive(Debug, Clone)]
    struct TestIndexer {
        name: String,
        calls: Arc<RwLock<Vec<String>>>,
    }

    impl TestIndexer {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                calls: Arc::new(RwLock::new(Vec::new())),
            }
        }

        fn record_call(&self, method: &str) {
            self.calls
                .write()
                .unwrap()
                .push(format!("{}::{}", self.name, method));
        }
    }

    impl Indexer for TestIndexer {
        type Error = Infallible;

        fn start(&mut self, _storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
            self.record_call("start");
            Ok(())
        }

        fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
            self.record_call("shutdown");
            Ok(())
        }

        fn pre_indexing(&self, _block_height: u64) -> std::result::Result<(), Self::Error> {
            self.record_call("pre_indexing");
            Ok(())
        }

        fn index_block(
            &self,
            _block: &Block,
            _block_outcome: &BlockOutcome,
        ) -> std::result::Result<(), Self::Error> {
            self.record_call("index_block");
            Ok(())
        }

        fn post_indexing(
            &self,
            _block_height: u64,
            _querier: Box<dyn QuerierProvider>,
        ) -> std::result::Result<(), Self::Error> {
            self.record_call("post_indexing");
            Ok(())
        }

        fn wait_for_finish(&self) {
            self.record_call("wait_for_finish");
        }
    }

    #[test]
    fn test_hooked_indexer_creation() {
        let indexer = HookedIndexer::new();
        assert!(!indexer.is_running());
        assert_eq!(indexer.indexer_count(), 0);
    }

    #[test]
    fn test_add_indexers() {
        let mut hooked_indexer = HookedIndexer::new();
        let test_indexer1 = TestIndexer::new("test1");
        let test_indexer2 = TestIndexer::new("test2");

        hooked_indexer.add_indexer(test_indexer1);
        hooked_indexer.add_indexer(test_indexer2);

        assert_eq!(hooked_indexer.indexer_count(), 2);
    }

    #[test]
    fn test_start_and_shutdown() {
        let mut hooked_indexer = HookedIndexer::new();
        let test_indexer = TestIndexer::new("test");
        let calls_tracker = test_indexer.calls.clone();

        hooked_indexer.add_indexer(test_indexer);

        let storage = MockStorage::new();

        // Test start
        assert!(!hooked_indexer.is_running());
        hooked_indexer.start(&storage).unwrap();
        assert!(hooked_indexer.is_running());

        // Test shutdown
        hooked_indexer.shutdown().unwrap();
        assert!(!hooked_indexer.is_running());

        let calls = calls_tracker.read().unwrap();
        assert_eq!(*calls, vec!["test::start", "test::shutdown"]);
    }

    #[test]
    fn test_double_start_fails() {
        let mut hooked_indexer = HookedIndexer::new();
        let storage = MockStorage::new();

        hooked_indexer.start(&storage).unwrap();
        let result = hooked_indexer.start(&storage);

        assert!(matches!(result, Err(HookedIndexerError::AlreadyRunning)));
    }

    #[test]
    fn test_operations_when_not_running() {
        let hooked_indexer = HookedIndexer::new();

        let result = hooked_indexer.pre_indexing(1);
        assert!(matches!(result, Err(HookedIndexerError::NotRunning)));
    }

    #[test]
    fn test_extensions_api() {
        let context = IndexerContext::new();

        // Test Extensions API - much simpler than TypedMapKey!
        context.data().write().unwrap().insert(42i32);
        context.data().write().unwrap().insert("hello".to_string());

        assert_eq!(context.data().read().unwrap().get::<i32>(), Some(&42));
        assert_eq!(
            context.data().read().unwrap().get::<String>(),
            Some(&"hello".to_string())
        );
        assert_eq!(context.data().read().unwrap().get::<u64>(), None);
    }
}
