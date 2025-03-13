use {
    async_trait::async_trait,
    grug_types::{BlockInfo, Query, QueryResponse},
};

#[async_trait]
pub trait Provider {
    type Error;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error>;

    async fn query_block(&self, height: Option<u64>) -> Result<BlockInfo, Self::Error>;
}
