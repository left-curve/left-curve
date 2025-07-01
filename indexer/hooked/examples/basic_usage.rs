use {
    grug_app::{Indexer, IndexerResult, QuerierProvider},
    grug_types::{Block, BlockOutcome, MockStorage, Storage},
    indexer_hooked::HookedIndexer,
};

/// Example indexer that logs blockchain events
#[derive(Debug, Clone)]
pub struct LoggingIndexer {
    prefix: String,
}

impl LoggingIndexer {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    fn log(&self, message: &str) {
        println!("[{}] {}", self.prefix, message);
    }
}

impl Indexer for LoggingIndexer {
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        self.log("🚀 Starting indexer");
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        self.log("🛑 Shutting down indexer");
        Ok(())
    }

    fn pre_indexing(&self, block_height: u64) -> IndexerResult<()> {
        self.log(&format!("📝 Pre-indexing block {block_height}"));
        Ok(())
    }

    fn index_block(&self, block: &Block, _outcome: &BlockOutcome) -> IndexerResult<()> {
        self.log(&format!(
            "🔍 Indexing block {} with {} txs",
            block.info.height,
            block.txs.len()
        ));
        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        _querier: &dyn QuerierProvider,
    ) -> IndexerResult<()> {
        self.log(&format!("✅ Post-indexing block {block_height}"));
        // With reference approach, we can easily use the querier here
        // but can't pass it to async tasks
        Ok(())
    }

    fn wait_for_finish(&self) {
        self.log("⏳ Waiting for indexer to finish");
    }
}

/// Example metrics indexer that tracks performance
#[derive(Debug, Default)]
pub struct MetricsIndexer {
    blocks_processed: std::sync::atomic::AtomicU64,
}

impl Indexer for MetricsIndexer {
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        println!("📊 Metrics indexer started");
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        let total = self
            .blocks_processed
            .load(std::sync::atomic::Ordering::SeqCst);
        println!("📊 Metrics indexer shutting down. Total blocks processed: {total}");
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> IndexerResult<()> {
        Ok(())
    }

    fn index_block(&self, _block: &Block, _outcome: &BlockOutcome) -> IndexerResult<()> {
        self.blocks_processed
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: &dyn QuerierProvider,
    ) -> IndexerResult<()> {
        Ok(())
    }

    fn wait_for_finish(&self) {
        println!("📊 Metrics indexer finished");
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
        _querier: &dyn QuerierProvider,
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
    println!("🎯 HookedIndexer Demo - Reference-based QuerierProvider");
    println!("This example shows how to compose multiple indexers with shared querier references");

    // Create our composite indexer
    let mut hooked_indexer = HookedIndexer::new();
    hooked_indexer
        .add_indexer(LoggingIndexer::new("MAIN"))
        .add_indexer(LoggingIndexer::new("BACKUP"))
        .add_indexer(MetricsIndexer::default())
        .add_indexer(DataIndexer);

    println!("\n📊 Indexer composition:");
    println!("  - {} indexers registered", hooked_indexer.indexer_count());
    println!("  - Running: {}", hooked_indexer.is_running());

    // Simulate the indexer lifecycle
    let mock_storage = MockStorage::new();

    hooked_indexer.start(&mock_storage)?;
    println!("  - Running: {}", hooked_indexer.is_running());

    // Simulate indexing a block
    let block = Block {
        info: grug_types::BlockInfo {
            height: 1,
            timestamp: grug_types::Timestamp::from_seconds(1234567890),
            hash: grug_types::Hash256::ZERO,
        },
        txs: vec![],
    };
    let outcome = BlockOutcome {
        app_hash: grug_types::Hash256::ZERO,
        cron_outcomes: vec![],
        tx_outcomes: vec![],
    };

    hooked_indexer.pre_indexing(1)?;
    hooked_indexer.index_block(&block, &outcome)?;

    // Create a mock querier for post_indexing
    let mock_querier = DummyQuerier;
    // With reference approach, all indexers get the same querier reference
    hooked_indexer.post_indexing(1, &mock_querier)?;

    hooked_indexer.wait_for_finish();
    hooked_indexer.shutdown()?;

    println!("\n✨ Demo completed successfully!");
    println!("All indexers shared the same querier reference efficiently");

    Ok(())
}
