use {
    crate::{Block, BlockOutcome},
    async_trait::async_trait,
};

#[async_trait]
pub trait BlockClient {
    type Error;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error>;

    async fn query_block_outcome(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error>;
}
