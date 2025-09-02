use {
    crate::{Indexer, IndexerError, IndexerResult, QuerierProvider},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Block, BlockOutcome},
    serde::{Deserialize, Serialize},
    std::{
        convert::Infallible,
        fmt::{self, Display},
        sync::Arc,
    },
};

/// This is a null indexer that does nothing.
#[derive(Debug, Clone, Default)]
pub struct NullIndexer;

impl Indexer for NullIndexer {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> IndexerResult<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> IndexerResult<()> {
        Ok(())
    }

    fn pre_indexing(
        &self,
        _block_height: u64,
        _ctx: &mut crate::IndexerContext,
    ) -> IndexerResult<()> {
        Ok(())
    }

    fn index_block(
        &self,
        _block: &Block,
        _block_outcome: &BlockOutcome,
        _ctx: &mut crate::IndexerContext,
    ) -> IndexerResult<()> {
        Ok(())
    }

    fn post_indexing(
        &self,
        _block_height: u64,
        _querier: Arc<dyn QuerierProvider>,
        _ctx: &mut crate::IndexerContext,
    ) -> IndexerResult<()> {
        Ok(())
    }

    fn wait_for_finish(&self) -> IndexerResult<()> {
        Ok(())
    }
}

/// An error type that is never encountered.
/// Used in conjunction with [`NullIndexer`](crate::NullIndexer).
#[derive(Debug)]
pub struct NullIndexerError(Infallible);

impl Display for NullIndexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NullIndexerError> for IndexerError {
    fn from(err: NullIndexerError) -> Self {
        IndexerError::Generic(err.to_string())
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, Eq, PartialEq)]
pub struct HttpRequestDetails {
    pub remote_ip: Option<String>,
    pub peer_ip: Option<String>,
}
