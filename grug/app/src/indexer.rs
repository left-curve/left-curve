use {
    crate::{AppError, Indexer},
    grug_types::{BlockInfo, BlockOutcome, Tx, TxOutcome},
    std::{
        convert::Infallible,
        fmt::{self, Display},
    },
};

/// This is a null indexer that does nothing.
#[derive(Debug, Clone, Default)]
pub struct NullIndexer;

impl Indexer for NullIndexer {
    type Error = NullIndexerError;

    fn start(&mut self) -> Result<(), Self::Error> {
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

/// An error type that is never encountered.
/// Used in conjunction with [`NullIndexer`](crate::NullIndexer).
pub struct NullIndexerError(Infallible);

impl Display for NullIndexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NullIndexerError> for AppError {
    fn from(err: NullIndexerError) -> Self {
        AppError::Indexer(err.to_string())
    }
}
