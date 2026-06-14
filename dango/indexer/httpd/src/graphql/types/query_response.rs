use {async_graphql::SimpleObject, dango_primitives::QueryResponse};

#[derive(SimpleObject)]
pub struct QueryResponseWithBlockHeight {
    pub response: QueryResponse,
    pub block_height: u64,
}
