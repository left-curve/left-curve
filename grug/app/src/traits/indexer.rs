use grug_types::{Block, BlockOutcome, Storage};

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
pub trait Indexer {
    type Error;

    /// Called when initializing the indexer, allowing for DB migration if needed
    fn start<S>(&mut self, _storage: &S) -> Result<(), Self::Error>
    where
        S: Storage,
    {
        Ok(())
    }

    /// Called when terminating the indexer, allowing for DB transactions to be committed
    fn shutdown(&mut self) -> Result<(), Self::Error>;

    /// Called when indexing a block, allowing to create a new DB transaction
    fn pre_indexing(&self, block_height: u64) -> Result<(), Self::Error>;

    /// Called when indexing the block, happens at the end of the block creation
    fn index_block(&self, block: &Block, block_outcome: &BlockOutcome) -> Result<(), Self::Error>;

    /// Called after indexing the block, allowing for DB transactions to be committed
    fn post_indexing(&self, block_height: u64) -> Result<(), Self::Error>;
}

/// NOTE: Not adding a error trait type on purpose as we use this as `&dyn IndexerBatch`
pub trait IndexerBatch {
    fn block(&self, block_height: u64) -> Result<BlockAndBlockOutcome, Box<dyn std::error::Error>>;
}

pub struct BlockAndBlockOutcome {
    pub block: Block,
    pub block_outcome: BlockOutcome,
}
