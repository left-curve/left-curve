use {async_graphql::*, block::BlockSubscription};

pub mod block;

#[derive(MergedSubscription, Default)]
pub struct Subscription(BlockSubscription);
