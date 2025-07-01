use {
    crate::{HookedIndexerError, Result},
    grug_app::{Indexer, QuerierProvider},
    grug_types::{GenericResult, Query, QueryResponse, Storage},
};

/// A QuerierProvider that can be cloned by copying its behavior
/// This is used to work around the ownership issues in post_indexing
pub struct QuerierProviderClone {
    // For now, this is a simplified implementation
    // In a real implementation, you might store the necessary data to reconstruct queries
}

impl QuerierProviderClone {
    pub fn from_ref(_querier: &dyn QuerierProvider) -> Self {
        // LIMITATION: We can't easily clone QuerierProvider state
        // This is a placeholder implementation
        Self {}
    }
}

impl QuerierProvider for QuerierProviderClone {
    fn do_query_chain(&self, _req: Query, _query_depth: usize) -> GenericResult<QueryResponse> {
        // LIMITATION: This is a no-op implementation
        // In a production system, you would need to either:
        // 1. Change the Indexer trait to not take ownership of QuerierProvider
        // 2. Implement proper cloning of QuerierProvider state
        // 3. Use a different approach like shared references
        Err("QuerierProvider cloning not implemented".into())
    }
}

/// Adapter that allows any Indexer to be used with HookedIndexer
/// Converts the indexer's error type to HookedIndexerError
pub struct IndexerAdapter<I> {
    indexer: std::sync::Mutex<I>,
}

impl<I> IndexerAdapter<I> {
    /// Create a new IndexerAdapter
    pub fn new(indexer: I) -> Self {
        Self {
            indexer: std::sync::Mutex::new(indexer),
        }
    }
}

// Explicitly implement Send and Sync for IndexerAdapter
unsafe impl<I> Send for IndexerAdapter<I> where I: Send {}
unsafe impl<I> Sync for IndexerAdapter<I> where I: Send + Sync {}

impl<I> Indexer for IndexerAdapter<I>
where
    I: Indexer + Send + Sync,
    I::Error: Into<HookedIndexerError>,
{
    type Error = HookedIndexerError;

    fn start(&mut self, storage: &dyn Storage) -> Result<()> {
        // Get mutable access to the inner indexer
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

    fn post_indexing(&self, block_height: u64, querier: Box<dyn QuerierProvider>) -> Result<()> {
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

/// A middleware that wraps an indexer with logging capabilities
pub struct LoggingMiddleware<I> {
    indexer: I,
    #[allow(dead_code)] // Used when tracing feature is enabled
    name: String,
}

impl<I> LoggingMiddleware<I> {
    /// Create a new LoggingMiddleware
    pub fn new(indexer: I, name: String) -> Self {
        Self { indexer, name }
    }
}

impl<I> Indexer for LoggingMiddleware<I>
where
    I: Indexer,
{
    type Error = I::Error;

    fn start(&mut self, storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::info!("Starting indexer: {}", self.name);

        let result = self.indexer.start(storage);

        #[cfg(feature = "tracing")]
        if result.is_err() {
            tracing::error!("Failed to start indexer: {}", self.name);
        }

        result
    }

    fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::info!("Shutting down indexer: {}", self.name);

        let result = self.indexer.shutdown();

        #[cfg(feature = "tracing")]
        if result.is_err() {
            tracing::error!("Failed to shutdown indexer: {}", self.name);
        }

        result
    }

    fn pre_indexing(&self, block_height: u64) -> std::result::Result<(), Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Pre-indexing block {} with indexer: {}",
            block_height,
            self.name
        );

        let result = self.indexer.pre_indexing(block_height);

        #[cfg(feature = "tracing")]
        if result.is_err() {
            tracing::error!(
                "Failed to pre-index block {} with indexer: {}",
                block_height,
                self.name
            );
        }

        result
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
    ) -> std::result::Result<(), Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Indexing block with indexer: {}", self.name);

        let result = self.indexer.index_block(block, block_outcome);

        #[cfg(feature = "tracing")]
        if result.is_err() {
            tracing::error!("Failed to index block with indexer: {}", self.name);
        }

        result
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: Box<dyn QuerierProvider>,
    ) -> std::result::Result<(), Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Post-indexing block {} with indexer: {}",
            block_height,
            self.name
        );

        let result = self.indexer.post_indexing(block_height, querier);

        #[cfg(feature = "tracing")]
        if result.is_err() {
            tracing::error!(
                "Failed to post-index block {} with indexer: {}",
                block_height,
                self.name
            );
        }

        result
    }

    fn wait_for_finish(&self) {}
}

/// A middleware that records metrics for indexer operations
#[cfg(feature = "metrics")]
pub struct MetricsMiddleware<I> {
    indexer: I,
    #[allow(dead_code)] // Used when metrics feature is enabled
    name: String,
}

#[cfg(feature = "metrics")]
impl<I> MetricsMiddleware<I> {
    /// Create a new MetricsMiddleware
    pub fn new(indexer: I, name: String) -> Self {
        Self { indexer, name }
    }
}

#[cfg(feature = "metrics")]
impl<I> Indexer for MetricsMiddleware<I>
where
    I: Indexer,
{
    type Error = I::Error;

    fn start(&mut self, storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
        let start_time = std::time::Instant::now();
        let result = self.indexer.start(storage);
        let duration = start_time.elapsed();

        metrics::histogram!("indexer_start_duration_seconds", "indexer" => self.name.clone())
            .record(duration.as_secs_f64());

        result
    }

    fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
        let start_time = std::time::Instant::now();
        let result = self.indexer.shutdown();
        let duration = start_time.elapsed();

        metrics::histogram!("indexer_shutdown_duration_seconds", "indexer" => self.name.clone())
            .record(duration.as_secs_f64());

        result
    }

    fn pre_indexing(&self, block_height: u64) -> std::result::Result<(), Self::Error> {
        let start_time = std::time::Instant::now();
        let result = self.indexer.pre_indexing(block_height);
        let duration = start_time.elapsed();

        metrics::histogram!("indexer_pre_indexing_duration_seconds", "indexer" => self.name.clone())
            .record(duration.as_secs_f64());

        result
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
    ) -> std::result::Result<(), Self::Error> {
        let start_time = std::time::Instant::now();
        let result = self.indexer.index_block(block, block_outcome);
        let duration = start_time.elapsed();

        metrics::histogram!("indexer_index_block_duration_seconds", "indexer" => self.name.clone())
            .record(duration.as_secs_f64());

        result
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: Box<dyn QuerierProvider>,
    ) -> std::result::Result<(), Self::Error> {
        let start_time = std::time::Instant::now();
        let result = self.indexer.post_indexing(block_height, querier);
        let duration = start_time.elapsed();

        metrics::histogram!("indexer_post_indexing_duration_seconds", "indexer" => self.name.clone())
            .record(duration.as_secs_f64());

        result
    }

    fn wait_for_finish(&self) {}
}

/// A no-op indexer for testing purposes
pub struct NoOpIndexer;

impl Indexer for NoOpIndexer {
    type Error = std::convert::Infallible;

    fn start(&mut self, _storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn index_block(
        &self,
        _block: &grug_types::Block,
        _block_outcome: &grug_types::BlockOutcome,
    ) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: Box<dyn QuerierProvider>,
    ) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn wait_for_finish(&self) {}
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::sync::{Arc, RwLock},
    };

    struct TestIndexer {
        calls: Arc<RwLock<Vec<String>>>,
    }

    impl TestIndexer {
        fn new() -> Self {
            Self {
                calls: Arc::new(RwLock::new(Vec::new())),
            }
        }

        fn record_call(&self, method: &str) {
            self.calls.write().unwrap().push(method.to_string());
        }
    }

    impl Indexer for TestIndexer {
        type Error = std::convert::Infallible;

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
    fn test_indexer_adapter() {
        let indexer = TestIndexer::new();
        let mut adapter = IndexerAdapter::new(indexer);
        let storage = grug_types::MockStorage::new();

        // Test that the adapter works
        adapter.start(&storage as &dyn Storage).unwrap();
        adapter.shutdown().unwrap();
    }

    #[test]
    fn test_logging_middleware() {
        let indexer = TestIndexer::new();
        let mut middleware = LoggingMiddleware::new(indexer, "test".to_string());
        let storage = grug_types::MockStorage::new();

        // Test that the middleware works
        middleware.start(&storage).unwrap();
        middleware.shutdown().unwrap();
    }

    #[test]
    fn test_noop_indexer() {
        let mut indexer = NoOpIndexer;
        let storage = grug_types::MockStorage::new();

        // Test that all operations succeed
        indexer.start(&storage).unwrap();
        indexer.shutdown().unwrap();
        indexer.pre_indexing(1).unwrap();
    }
}
