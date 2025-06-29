use grug_types::{Block, BlockOutcome, Storage};

use crate::QuerierProvider;

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
pub trait Indexer {
    type Error: ToString;

    /// Called when initializing the indexer, allowing for DB migration if needed
    fn start(&mut self, _storage: &dyn Storage) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called when terminating the indexer, allowing for DB transactions to be committed
    fn shutdown(&mut self) -> Result<(), Self::Error>;

    /// Called when indexing a block, allowing to create a new DB transaction
    fn pre_indexing(&self, block_height: u64) -> Result<(), Self::Error>;

    /// Called when indexing the block, happens at the end of the block creation
    fn index_block(&self, block: &Block, block_outcome: &BlockOutcome) -> Result<(), Self::Error>;

    /// Called after indexing the block, allowing for DB transactions to be committed
    fn post_indexing(
        &self,
        block_height: u64,
        querier: Box<dyn QuerierProvider>,
    ) -> Result<(), Self::Error>;
}
