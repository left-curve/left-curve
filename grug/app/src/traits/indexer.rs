use grug_types::{Block, BlockOutcome, Storage};

use crate::{IndexerError, QuerierProvider};

/// Result type for indexer operations
pub type IndexerResult<T> = Result<T, IndexerError>;

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
pub trait Indexer {
    /// Called when initializing the indexer, allowing for DB migration if needed
    fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        Ok(())
    }

    /// Called when terminating the indexer, allowing for DB transactions to be committed
    fn shutdown(&mut self) -> IndexerResult<()>;

    /// Called when indexing a block, allowing to create a new DB transaction
    fn pre_indexing(&self, block_height: u64) -> IndexerResult<()>;

    /// Called when indexing the block, happens at the end of the block creation
    fn index_block(&self, block: &Block, block_outcome: &BlockOutcome) -> IndexerResult<()>;

    /// Called after indexing the block, allowing for DB transactions to be committed
    fn post_indexing(
        &self,
        block_height: u64,
        querier: Box<dyn QuerierProvider>,
    ) -> IndexerResult<()>;

    /// Wait for the indexer to finish indexing
    fn wait_for_finish(&self);
}
