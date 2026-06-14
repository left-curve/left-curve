use {
    crate::IndexerError,
    async_trait::async_trait,
    dango_primitives::{Block, BlockOutcome, Config, Json, Storage},
    std::any::type_name,
};

/// Result type for indexer operations
pub type IndexerResult<T> = Result<T, IndexerError>;

/// This is the trait that the indexer must implement. It is used by the Grug core to index blocks
#[async_trait]
pub trait Indexer: Send + Sync {
    fn name(&self) -> &'static str {
        type_name::<Self>()
    }

    /// Called when initializing the indexer, allowing for DB migration if needed
    async fn start(&mut self, _storage: &dyn Storage) -> IndexerResult<()> {
        Ok(())
    }

    /// Called when terminating the indexer, allowing for DB transactions to be committed
    async fn shutdown(&mut self) -> IndexerResult<()> {
        Ok(())
    }

    /// Called when indexing a block, allowing to create a new DB transaction
    async fn pre_indexing(&self, _block_height: u64) -> IndexerResult<()> {
        Ok(())
    }

    /// Called when indexing the block, happens at the end of the block creation
    async fn index_block(
        &self,
        _block: &Block,
        _block_outcome: &BlockOutcome,
    ) -> IndexerResult<()> {
        Ok(())
    }

    /// Called after indexing the block, allowing for DB transactions to be committed
    async fn post_indexing(
        &self,
        _block_height: u64,
        _cfg: Config,
        _app_cfg: Json,
    ) -> IndexerResult<()> {
        Ok(())
    }

    /// Wait for the indexer to finish indexing
    async fn wait_for_finish(&self) -> IndexerResult<()> {
        Ok(())
    }

    async fn last_indexed_block_height(&self) -> IndexerResult<Option<u64>> {
        Ok(None)
    }
}
