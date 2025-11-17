use {
    async_graphql::*, block::BlockSubscription, event::EventSubscription, grug::GrugSubscription,
    message::MessageSubscription, transaction::TransactionSubscription,
};

pub mod block;
pub mod event;
pub mod grug;
pub mod message;
pub mod transaction;

pub const MAX_PAST_BLOCKS: usize = 100;

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    BlockSubscription,
    TransactionSubscription,
    MessageSubscription,
    EventSubscription,
    GrugSubscription,
);
