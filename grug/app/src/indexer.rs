use {
    crate::Indexer,
    grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome},
    std::convert::Infallible,
};

/// This is a null indexer that does nothing.
#[derive(Debug, Clone, Default)]
pub struct NullIndexer;

impl NullIndexer {
    pub fn new() -> NullIndexer {
        NullIndexer {}
    }
}

impl Indexer for NullIndexer {
    type Error = Infallible;

    fn start<S>(&mut self, _storage: &S) -> Result<(), Self::Error> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn pre_indexing(&self, _block_height: u64) -> Result<(), Self::Error> {
        Ok(())
    }

    fn index_block(
        &self,
        _block: &BlockInfo,
        _block_outcome: &BlockOutcome,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn index_transaction(
        &self,
        _block: &BlockInfo,
        _tx: Tx,
        _tx_outcome: TxOutcome,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn post_indexing(&self, _block_height: u64) -> Result<(), Self::Error> {
        Ok(())
    }
}
