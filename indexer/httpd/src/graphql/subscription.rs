use {
    async_graphql::*, block::BlockSubscription, event::EventSubscription,
    message::MessageSubscription, transaction::TransactionSubscription,
};

pub mod block;
pub mod event;
pub mod message;
pub mod transaction;

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    BlockSubscription,
    TransactionSubscription,
    MessageSubscription,
    EventSubscription,
);
