use {async_graphql::SimpleObject, grug_types::QueryResponse};

#[derive(SimpleObject)]
pub struct QueryResponseWithBlockHeight {
    pub response: QueryResponse,
    pub block_height: u64,
}
