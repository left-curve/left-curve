pub use typedmap::TypedDashMap as TypeMap;
use {grug_app::Indexer, grug_types::Storage};

// Re-export modules for easier access
pub mod context;
pub mod error;
pub mod middleware;

pub use {
    context::IndexerContext,
    error::{HookedIndexerError, Result},
};

// DynIndexer trait removed - we can now use Indexer directly since it's dyn-compatible

/// A composable indexer that can own multiple indexers and coordinate between them
pub struct HookedIndexer {
    /// List of registered indexers with error conversion
    indexers: Vec<Box<dyn Indexer<Error = HookedIndexerError>>>,
    /// Shared context that gets passed to middleware (not directly to indexers)
    context: IndexerContext,
    /// Whether the indexer is currently running
    is_running: bool,
}

impl HookedIndexer {
    /// Create a new HookedIndexer with empty indexers
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
        let adapter = middleware::IndexerAdapter::new(indexer);
        self.indexers.push(Box::new(adapter));
        self
    }

    /// Add a boxed Indexer directly
    pub fn add_boxed_indexer(
        &mut self,
        indexer: Box<dyn Indexer<Error = HookedIndexerError>>,
    ) -> &mut Self {
        self.indexers.push(indexer);
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
            // For others, we need to create a clone/wrapper
            if i == self.indexers.len() - 1 {
                indexer.post_indexing(block_height, querier)?;
                break;
            } else {
                // Create a wrapper that clones the querier functionality
                let querier_clone = middleware::QuerierProviderClone::from_ref(querier.as_ref());
                indexer.post_indexing(block_height, Box::new(querier_clone))?;
            }
        }

        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug_types::MockStorage,
        std::{
            convert::Infallible,
            sync::{Arc, RwLock},
        },
        typedmap::TypedMapKey,
    };

    // Test types for TypedMapKey
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestKey(String);

    impl TypedMapKey for TestKey {
        type Value = i32;
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestStringKey(String);

    impl TypedMapKey for TestStringKey {
        type Value = String;
    }

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
            _block: &grug_types::Block,
            _block_outcome: &grug_types::BlockOutcome,
        ) -> std::result::Result<(), Self::Error> {
            self.record_call("index_block");
            Ok(())
        }

        fn post_indexing(
            &self,
            _block_height: u64,
            _querier: Box<dyn grug_app::QuerierProvider>,
        ) -> std::result::Result<(), Self::Error> {
            self.record_call("post_indexing");
            Ok(())
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
    fn test_typemap() {
        let typemap = TypeMap::new();

        // Test insert and get - much cleaner with TypedMapKey!
        typemap.insert(TestKey("number".to_string()), 42);
        typemap.insert(TestStringKey("message".to_string()), "hello".to_string());

        assert_eq!(
            typemap
                .get(&TestKey("number".to_string()))
                .map(|v| *v.value()),
            Some(42)
        );
        assert_eq!(
            typemap
                .get(&TestStringKey("message".to_string()))
                .map(|v| v.value().clone()),
            Some("hello".to_string())
        );
        assert!(typemap.get(&TestKey("missing".to_string())).is_none());

        // Test contains
        assert!(typemap.contains_key(&TestKey("number".to_string())));
        assert!(typemap.contains_key(&TestStringKey("message".to_string())));
        assert!(!typemap.contains_key(&TestKey("missing".to_string())));

        // Test remove
        let removed = typemap.remove(&TestKey("number".to_string()));
        assert!(removed.is_some());
        assert!(!typemap.contains_key(&TestKey("number".to_string())));
    }

    #[test]
    fn test_context_data_sharing() {
        let context = IndexerContext::new();

        // Test data insertion and retrieval - much cleaner!
        context.data().insert(TestKey("count".to_string()), 123);
        context
            .data()
            .insert(TestStringKey("name".to_string()), "test".to_string());

        assert_eq!(
            context
                .data()
                .get(&TestKey("count".to_string()))
                .map(|v| *v.value()),
            Some(123)
        );
        assert_eq!(
            context
                .data()
                .get(&TestStringKey("name".to_string()))
                .map(|v| v.value().clone()),
            Some("test".to_string())
        );
        assert!(
            context
                .data()
                .get(&TestKey("missing".to_string()))
                .is_none()
        );
    }
}
