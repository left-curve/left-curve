use {
    grug_app::{Indexer, IndexerResult, QuerierProvider},
    grug_types::{Block, BlockOutcome, MockStorage, Storage},
    indexer_hooked::HookedIndexer,
};

/// Example logging indexer that prints operations
#[derive(Debug, Clone)]
struct LoggingIndexer {
    name: String,
}

impl LoggingIndexer {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl Indexer for LoggingIndexer {
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        println!("{}: Starting indexer", self.name);
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        println!("{}: Shutting down indexer", self.name);
        Ok(())
    }

    fn pre_indexing(&self, block_height: u64) -> IndexerResult<()> {
        println!("{}: Pre-indexing block {}", self.name, block_height);
        Ok(())
    }

    fn index_block(&self, block: &Block, _block_outcome: &BlockOutcome) -> IndexerResult<()> {
        println!("{}: Indexing block {}", self.name, block.info.height);
        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        _querier: Box<dyn QuerierProvider>,
    ) -> IndexerResult<()> {
        println!("{}: Post-indexing block {}", self.name, block_height);
        Ok(())
    }

    fn wait_for_finish(&self) {
        println!("{}: Waiting for indexer to finish", self.name);
    }
}

/// Example metrics indexer that tracks statistics
#[derive(Debug, Default)]
struct MetricsIndexer {
    total_blocks: std::sync::Arc<std::sync::atomic::AtomicU64>,
    total_txs: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Indexer for MetricsIndexer {
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        println!("[Metrics] Starting metrics collection");
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        let blocks = self.total_blocks.load(std::sync::atomic::Ordering::Relaxed);
        let txs = self.total_txs.load(std::sync::atomic::Ordering::Relaxed);
        println!("[Metrics] Final stats: {blocks} blocks, {txs} transactions");
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> IndexerResult<()> {
        // Pre-processing metrics can be added here
        Ok(())
    }

    fn index_block(&self, block: &Block, _block_outcome: &BlockOutcome) -> IndexerResult<()> {
        self.total_blocks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_txs
            .fetch_add(block.txs.len() as u64, std::sync::atomic::Ordering::Relaxed);

        let blocks = self.total_blocks.load(std::sync::atomic::Ordering::Relaxed);
        let txs = self.total_txs.load(std::sync::atomic::Ordering::Relaxed);
        println!("[Metrics] Updated stats: {blocks} blocks, {txs} transactions");
        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: Box<dyn QuerierProvider>,
    ) -> IndexerResult<()> {
        // Post-processing metrics can be added here
        Ok(())
    }

    fn wait_for_finish(&self) {
        println!("[Metrics] Metrics collection complete");
    }
}

/// Example data indexer that stores information
#[derive(Debug, Default)]
struct DataIndexer;

impl Indexer for DataIndexer {
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        println!("[Data] Initializing data storage");
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        println!("[Data] Persisting final data");
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> IndexerResult<()> {
        println!("[Data] Preparing data structures");
        Ok(())
    }

    fn index_block(&self, block: &Block, _block_outcome: &BlockOutcome) -> IndexerResult<()> {
        println!("[Data] Storing block {} data", block.info.height);
        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: Box<dyn QuerierProvider>,
    ) -> IndexerResult<()> {
        println!("[Data] Finalizing data storage");
        Ok(())
    }

    fn wait_for_finish(&self) {
        println!("[Data] Data storage complete");
    }
}

/// Dummy querier for testing
struct DummyQuerier;

impl QuerierProvider for DummyQuerier {
    fn do_query_chain(
        &self,
        _req: grug_types::Query,
        _query_depth: usize,
    ) -> grug_types::GenericResult<grug_types::QueryResponse> {
        Ok(grug_types::QueryResponse::WasmRaw(Some(
            grug_types::Binary::from(b"dummy response".to_vec()),
        )))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 HookedIndexer Example");

    // Create a hooked indexer and add multiple indexers
    let mut hooked_indexer = HookedIndexer::new();

    // Add different types of indexers
    hooked_indexer
        .add_indexer(LoggingIndexer::new("Logger"))
        .add_indexer(MetricsIndexer::default())
        .add_indexer(DataIndexer);

    println!("📊 Added {} indexers", hooked_indexer.indexer_count());

    // Test shared context functionality
    hooked_indexer
        .context()
        .data()
        .write()
        .unwrap()
        .insert(42i32);
    hooked_indexer
        .context()
        .data()
        .write()
        .unwrap()
        .insert("shared data".to_string());

    if let Some(value) = hooked_indexer.context().data().read().unwrap().get::<i32>() {
        println!("🔗 Shared integer: {value}");
    }

    if let Some(text) = hooked_indexer
        .context()
        .data()
        .read()
        .unwrap()
        .get::<String>()
    {
        println!("🔗 Shared string: {text}");
    }

    // Simulate indexing operations
    let storage = MockStorage::new();

    // Start the indexer
    hooked_indexer.start(&storage)?;

    // Simulate indexing a few blocks
    for height in 1..=3 {
        println!("\n--- Block {height} ---");

        hooked_indexer.pre_indexing(height)?;

        // Create a dummy block
        let block = Block {
            info: grug_types::BlockInfo {
                height,
                timestamp: grug_types::Timestamp::from_seconds(1234567890 + height as u128),
                hash: grug_types::Hash256::ZERO,
            },
            txs: vec![], // Empty for this example
        };
        let block_outcome = BlockOutcome {
            app_hash: grug_types::Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        hooked_indexer.index_block(&block, &block_outcome)?;

        let querier = DummyQuerier;
        hooked_indexer.post_indexing(height, Box::new(querier))?;
    }

    // Demonstrate error handling
    println!("\n--- Testing Error Handling ---");

    // Shutdown the indexer
    hooked_indexer.shutdown()?;

    // Try to use after shutdown (should fail)
    if let Err(e) = hooked_indexer.pre_indexing(999) {
        println!("❌ Expected error after shutdown: {e}");
    }

    println!("\n✅ Example completed successfully!");

    Ok(())
}
