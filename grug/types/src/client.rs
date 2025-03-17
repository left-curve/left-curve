use {
    crate::{BlockInfo, Query, QueryResponse},
    async_trait::async_trait,
};

#[async_trait]
pub trait Client {
    type Error;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error>;

    async fn query_block(&self, height: Option<u64>) -> Result<BlockInfo, Self::Error>;
}
