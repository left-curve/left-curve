use {
    async_graphql::*,
    indexer_httpd::graphql::subscription::{
        block::BlockSubscription, event::EventSubscription, message::MessageSubscription,
        transaction::TransactionSubscription,
    },
    transfer::TransferSubscription,
};

pub mod account;
pub mod transfer;

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    TransferSubscription,
    BlockSubscription,
    TransactionSubscription,
    MessageSubscription,
    EventSubscription,
);
