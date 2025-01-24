use {async_graphql::MergedObject, block::BlockQuery, message::MessageQuery};

pub mod block;
pub mod message;

#[derive(MergedObject, Default)]
pub struct Query(BlockQuery, MessageQuery);
