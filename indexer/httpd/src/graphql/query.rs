use async_graphql::MergedObject;
use block::BlockQuery;

pub mod block;
pub mod index;

#[derive(MergedObject, Default)]
pub struct Query(BlockQuery);
