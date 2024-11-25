use {
    super::{error, IndexerTrait},
    grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome},
};

/// This is a null indexer that does nothing.
#[derive(Debug, Clone)]
pub struct Indexer;

impl Indexer {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Indexer {})
    }
}

impl IndexerTrait for Indexer {
    fn shutdown(&mut self) -> error::Result<()> {
        Ok(())
    }

    fn start(&self) -> error::Result<()> {
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> error::Result<()> {
        Ok(())
    }

    fn index_block(&self, _block: &BlockInfo, _block_outcome: &BlockOutcome) -> error::Result<()> {
        Ok(())
    }

    fn index_transaction(
        &self,
        _block: &BlockInfo,
        _tx: &Tx,
        _tx_outcome: &TxOutcome,
    ) -> error::Result<()> {
        Ok(())
    }

    fn post_indexing(&self, _block_height: u64) -> error::Result<()> {
        Ok(())
    }
}
