use grug_app::{Indexer, IndexerResult};

// Re-export for convenience
pub use grug_app::IndexerError;

/// A composable indexer that can own multiple indexers and coordinate between them
pub struct HookedIndexer {
    /// List of registered indexers - no wrappers needed!
    indexers: Vec<Box<dyn Indexer + Send + Sync>>,
    /// Whether the indexer is currently running
    is_running: bool,
}

impl HookedIndexer {
    pub fn new() -> Self {
        Self {
            indexers: Vec::new(),
            is_running: false,
        }
    }

    /// Add an indexer to the composition
    pub fn add_indexer<I>(&mut self, indexer: I) -> &mut Self
    where
        I: Indexer + Send + Sync + 'static,
    {
        self.indexers.push(Box::new(indexer));
        self
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
    fn start(&mut self, storage: &dyn grug_types::Storage) -> IndexerResult<()> {
        if self.is_running {
            return Err(grug_app::IndexerError::AlreadyRunning);
        }

        // Call start on all indexers
        for indexer in &mut self.indexers {
            indexer.start(storage)?;
        }

        self.is_running = true;
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
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
            return Err(grug_app::IndexerError::Multiple(errors));
        }

        Ok(())
    }

    fn pre_indexing(&self, block_height: u64) -> IndexerResult<()> {
        if !self.is_running {
            return Err(grug_app::IndexerError::NotRunning);
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
    ) -> IndexerResult<()> {
        if !self.is_running {
            return Err(grug_app::IndexerError::NotRunning);
        }

        for indexer in &self.indexers {
            indexer.index_block(block, block_outcome)?;
        }

        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: &dyn grug_app::QuerierProvider,
    ) -> IndexerResult<()> {
        if !self.is_running {
            return Err(grug_app::IndexerError::NotRunning);
        }

        // All indexers get the same querier reference efficiently!
        for indexer in &self.indexers {
            indexer.post_indexing(block_height, querier)?;
        }

        Ok(())
    }

    fn wait_for_finish(&self) {
        for indexer in self.indexers.iter().rev() {
            indexer.wait_for_finish();
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, grug_types::MockStorage};

    #[derive(Default)]
    struct TestIndexer {
        calls: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
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

        fn pre_indexing(&self, _block_height: u64) -> IndexerResult<()> {
            self.record_call("pre_indexing");
            Ok(())
        }

        fn index_block(
            &self,
            _block: &grug_types::Block,
            _block_outcome: &grug_types::BlockOutcome,
        ) -> IndexerResult<()> {
            self.record_call("index_block");
            Ok(())
        }

        fn post_indexing(
            &self,
            _block_height: u64,
            _querier: &dyn grug_app::QuerierProvider,
        ) -> IndexerResult<()> {
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
        assert_eq!(indexer.indexer_count(), 0);
        assert!(!indexer.is_running());
    }

    #[test]
    fn test_add_indexers() {
        let mut hooked_indexer = HookedIndexer::new();

        hooked_indexer
            .add_indexer(TestIndexer::default())
            .add_indexer(TestIndexer::default());

        assert_eq!(hooked_indexer.indexer_count(), 2);
    }

    #[test]
    fn test_start_and_shutdown() {
        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer.add_indexer(TestIndexer::default());

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
        let storage = MockStorage::new();

        hooked_indexer.start(&storage).unwrap();

        // Second start should fail
        let result = hooked_indexer.start(&storage);
        assert!(matches!(
            result,
            Err(grug_app::IndexerError::AlreadyRunning)
        ));
    }

    #[test]
    fn test_operations_when_not_running() {
        let hooked_indexer = HookedIndexer::new();

        // Operations should fail when not running
        assert!(matches!(
            hooked_indexer.pre_indexing(1),
            Err(grug_app::IndexerError::NotRunning)
        ));

        let block = grug_types::Block {
            info: grug_types::BlockInfo {
                height: 1,
                timestamp: grug_types::Timestamp::from_seconds(123456789),
                hash: grug_types::Hash256::ZERO,
            },
            txs: vec![],
        };
        let outcome = grug_types::BlockOutcome {
            app_hash: grug_types::Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        assert!(matches!(
            hooked_indexer.index_block(&block, &outcome),
            Err(grug_app::IndexerError::NotRunning)
        ));
    }
}
