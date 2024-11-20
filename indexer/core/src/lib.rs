use grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome};

pub mod active_model;
pub mod blocking_indexer;
pub mod context;
pub mod error;
pub mod non_blocking_indexer;
pub mod null_indexer;

pub use context::Context;

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
pub trait IndexerTrait {
    fn new() -> Result<Self, anyhow::Error>
    where
        Self: Sized;
    fn start(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn shutdown(self) -> Result<(), anyhow::Error>;

    fn pre_indexing(&self, block_height: u64) -> Result<(), anyhow::Error>;

    fn index_block(
        &self,
        block: &BlockInfo,
        block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error>;

    fn index_transaction(
        &self,
        block: &BlockInfo,
        tx: &Tx,
        tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error>;

    fn post_indexing(&self, block_height: u64) -> Result<(), anyhow::Error>;
}
