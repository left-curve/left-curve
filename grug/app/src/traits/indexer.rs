use grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome};

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
pub trait Indexer {
    type Error: ToString;

    /// Called when initializing the indexer, allowing for DB migration if needed
    fn start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Called when terminating the indexer, allowing for DB transactions to be committed
    fn shutdown(&mut self) -> Result<(), Self::Error>;

    /// Called when indexing a block, allowing to create a new DB transaction
    fn pre_indexing(&self, block_height: u64) -> Result<(), Self::Error>;

    /// Called for each transaction, happens before `index_block` and might be called multiple
    /// times per block (for each transaction)
    fn index_transaction(
        &self,
        block: &BlockInfo,
        tx: Tx,
        tx_outcome: TxOutcome,
    ) -> Result<(), Self::Error>;

    /// Called when indexing the block, happens at the end of the block creation
    fn index_block(
        &self,
        block: &BlockInfo,
        block_outcome: &BlockOutcome,
    ) -> Result<(), Self::Error>;

    /// Called after indexing the block, allowing for DB transactions to be committed
    fn post_indexing(&self, block_height: u64) -> Result<(), Self::Error>;
}
