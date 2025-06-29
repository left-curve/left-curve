use {
    grug_app::{Indexer, QuerierProvider},
    grug_types::{Block, BlockOutcome, GenericResult, MockStorage, Query, QueryResponse, Storage},
    indexer_hooked::{HookedIndexer, Result},
    std::sync::{Arc, RwLock},
};

// Simple mock QuerierProvider for testing
#[derive(Debug)]
struct MockQuerierProvider;

impl MockQuerierProvider {
    fn new() -> Self {
        Self
    }
}

impl QuerierProvider for MockQuerierProvider {
    fn do_query_chain(&self, _req: Query, _query_depth: usize) -> GenericResult<QueryResponse> {
        // For this example, we don't need actual querying functionality
        // In a real implementation, you would handle the query appropriately
        Err("Mock querier - not implemented".to_string())
    }
}

// Example of a simple logging indexer
#[derive(Debug, Clone)]
struct LoggingIndexer {
    logs: Arc<RwLock<Vec<String>>>,
}

impl LoggingIndexer {
    fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[allow(dead_code)] // Public API method for users of LoggingIndexer
    fn get_logs(&self) -> Vec<String> {
        self.logs.read().unwrap().clone()
    }

    fn log(&self, message: String) {
        self.logs.write().unwrap().push(message);
    }
}

impl Indexer for LoggingIndexer {
    type Error = std::convert::Infallible;

    fn start(&mut self, _storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
        self.log("LoggingIndexer started".to_string());
        Ok(())
    }

    fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
        self.log("LoggingIndexer shut down".to_string());
        Ok(())
    }

    fn pre_indexing(&self, block_height: u64) -> std::result::Result<(), Self::Error> {
        self.log(format!("Pre-indexing block {block_height}"));
        Ok(())
    }

    fn index_block(
        &self,
        block: &Block,
        _block_outcome: &BlockOutcome,
    ) -> std::result::Result<(), Self::Error> {
        self.log(format!("Indexing block {}", block.info.height));
        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        _querier: Box<dyn grug_app::QuerierProvider>,
    ) -> std::result::Result<(), Self::Error> {
        self.log(format!("Post-indexing block {block_height}"));
        Ok(())
    }
}

// Example of a data processing indexer that counts blocks
#[derive(Debug, Clone)]
struct DataProcessorIndexer {
    processed_count: Arc<RwLock<u64>>,
}

impl DataProcessorIndexer {
    fn new() -> Self {
        Self {
            processed_count: Arc::new(RwLock::new(0)),
        }
    }

    fn get_processed_count(&self) -> u64 {
        *self.processed_count.read().unwrap()
    }
}

impl Indexer for DataProcessorIndexer {
    type Error = std::convert::Infallible;

    fn start(&mut self, _storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
        println!("DataProcessorIndexer initialized");
        Ok(())
    }

    fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
        println!(
            "DataProcessorIndexer shutting down with {} blocks processed",
            self.get_processed_count()
        );
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn index_block(
        &self,
        block: &Block,
        _block_outcome: &BlockOutcome,
    ) -> std::result::Result<(), Self::Error> {
        // Process the block data
        println!("Processing block {}", block.info.height);

        // Update our counter
        *self.processed_count.write().unwrap() += 1;

        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: Box<dyn grug_app::QuerierProvider>,
    ) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
}

// Example of a metrics indexer that tracks statistics
#[derive(Debug, Clone)]
struct MetricsIndexer {
    total_blocks: Arc<RwLock<u64>>,
    start_time: Arc<RwLock<Option<std::time::Instant>>>,
}

impl MetricsIndexer {
    fn new() -> Self {
        Self {
            total_blocks: Arc::new(RwLock::new(0)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    fn get_total_blocks(&self) -> u64 {
        *self.total_blocks.read().unwrap()
    }

    fn get_uptime(&self) -> Option<std::time::Duration> {
        self.start_time.read().unwrap().map(|start| start.elapsed())
    }
}

impl Indexer for MetricsIndexer {
    type Error = std::convert::Infallible;

    fn start(&mut self, _storage: &dyn Storage) -> std::result::Result<(), Self::Error> {
        *self.start_time.write().unwrap() = Some(std::time::Instant::now());
        println!("MetricsIndexer started");
        Ok(())
    }

    fn shutdown(&mut self) -> std::result::Result<(), Self::Error> {
        if let Some(uptime) = self.get_uptime() {
            println!("MetricsIndexer shutting down after {uptime:?} uptime");
        }
        println!("Total blocks indexed: {}", self.get_total_blocks());
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn index_block(
        &self,
        _block: &Block,
        _block_outcome: &BlockOutcome,
    ) -> std::result::Result<(), Self::Error> {
        *self.total_blocks.write().unwrap() += 1;
        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: Box<dyn grug_app::QuerierProvider>,
    ) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
}

fn main() -> Result<()> {
    println!("🔗 HookedIndexer Example");

    // Create our indexers
    let logging_indexer = LoggingIndexer::new();
    let processor_indexer = DataProcessorIndexer::new();
    let metrics_indexer = MetricsIndexer::new();

    // Keep references to check results later
    let logging_logs = logging_indexer.logs.clone();
    let processor_count = processor_indexer.processed_count.clone();
    let metrics_total = metrics_indexer.total_blocks.clone();

    // Create a composed indexer
    let mut hooked_indexer = HookedIndexer::new();
    hooked_indexer
        .add_indexer(logging_indexer)
        .add_indexer(processor_indexer)
        .add_indexer(metrics_indexer);

    println!(
        "Created HookedIndexer with {} indexers",
        hooked_indexer.indexer_count()
    );

    // Create some mock data
    let storage = MockStorage::new();

    // Start the indexer
    hooked_indexer.start(&storage)?;
    println!("✅ HookedIndexer started");

    // Simulate indexing some blocks
    for height in 1..=5 {
        let block = Block {
            info: grug_types::BlockInfo {
                height,
                timestamp: grug_types::Timestamp::from_seconds(1234567890 + height as u128),
                hash: grug_types::Hash256::ZERO,
            },
            txs: vec![],
        };

        let block_outcome = BlockOutcome {
            app_hash: grug_types::Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        // Run the indexing pipeline
        hooked_indexer.pre_indexing(height)?;
        hooked_indexer.index_block(&block, &block_outcome)?;

        // Create a mock querier for post_indexing
        let querier = MockQuerierProvider::new();
        hooked_indexer.post_indexing(height, Box::new(querier))?;

        println!("📦 Indexed block {height}");
    }

    // Check the context data
    let context = hooked_indexer.context();
    let data_count = context.data().len();
    println!(
        "Context has {} data items and {} metadata properties",
        data_count,
        context.metadata().properties.len()
    );

    // Shutdown
    hooked_indexer.shutdown()?;
    println!("🛑 HookedIndexer shut down");

    // Display results
    println!("\n📊 Results:");
    println!(
        "Logs from LoggingIndexer: {:?}",
        logging_logs.read().unwrap().clone()
    );
    println!(
        "Blocks processed by DataProcessorIndexer: {}",
        processor_count.read().unwrap()
    );
    println!(
        "Total blocks tracked by MetricsIndexer: {}",
        metrics_total.read().unwrap()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooked_indexer_composition() -> Result<()> {
        let logging_indexer = LoggingIndexer::new();
        let processor_indexer = DataProcessorIndexer::new();

        let logging_logs = logging_indexer.logs.clone();
        let processor_count = processor_indexer.processed_count.clone();

        let mut hooked_indexer = HookedIndexer::new();
        hooked_indexer
            .add_indexer(logging_indexer)
            .add_indexer(processor_indexer);

        let storage = MockStorage::new();
        hooked_indexer.start(&storage)?;

        // Simulate indexing a block
        let block = Block {
            info: grug_types::BlockInfo {
                height: 1,
                timestamp: grug_types::Timestamp::from_seconds(1234567890),
                hash: grug_types::Hash256::ZERO,
            },
            txs: vec![],
        };

        let block_outcome = BlockOutcome {
            app_hash: grug_types::Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        hooked_indexer.pre_indexing(1)?;
        hooked_indexer.index_block(&block, &block_outcome)?;

        let querier = MockQuerierProvider::new();
        hooked_indexer.post_indexing(1, Box::new(querier))?;

        hooked_indexer.shutdown()?;

        // Check that both indexers worked
        assert!(!logging_logs.read().unwrap().is_empty());
        assert_eq!(*processor_count.read().unwrap(), 1);

        Ok(())
    }

    #[test]
    fn test_context_usage() -> Result<()> {
        use typedmap::TypedMapKey;

        // Define a test key type for typed storage
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct TestCounter;

        impl TypedMapKey for TestCounter {
            type Value = i32;
        }

        let mut hooked_indexer = HookedIndexer::new();

        // Add some data to the context
        hooked_indexer
            .context_mut()
            .set_property("test_key".to_string(), "test_value".to_string());
        hooked_indexer.context().data().insert(TestCounter, 42);

        // Check that the context works
        assert_eq!(
            hooked_indexer.context().get_property("test_key"),
            Some("test_value")
        );
        assert_eq!(
            hooked_indexer
                .context()
                .data()
                .get(&TestCounter)
                .map(|v| *v.value()),
            Some(42)
        );

        Ok(())
    }
}
