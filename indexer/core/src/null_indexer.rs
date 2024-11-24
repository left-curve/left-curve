use grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome};

use super::IndexerTrait;

/// This is a null indexer that does nothing.
#[derive(Debug, Clone)]
pub struct Indexer;

impl Indexer {
    pub fn new() -> Result<Self, anyhow::Error> {
        Ok(Indexer {})
    }
}

impl IndexerTrait for Indexer {
    fn shutdown(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn start(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn index_block(
        &self,
        _block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn index_transaction(
        &self,
        _block: &BlockInfo,
        _tx: &Tx,
        _tx_outcome: &TxOutcome,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn post_indexing(&self, _block_height: u64) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
