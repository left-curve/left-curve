use {
    super::{
        block::BlockSubscription, event::EventSubscription, grug::GrugSubscription,
        message::MessageSubscription, transaction::TransactionSubscription,
    },
    async_graphql::*,
};

#[derive(MergedSubscription, Default)]
pub struct IndexerSubscription(
    BlockSubscription,
    TransactionSubscription,
    MessageSubscription,
    EventSubscription,
    GrugSubscription,
);
