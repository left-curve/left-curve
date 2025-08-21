use {
    account::AccountSubscription,
    async_graphql::*,
    dango_indexer_clickhouse::httpd::graphql::subscription::{
        candle::CandleSubscription, trade::TradeSubscription,
    },
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
    AccountSubscription,
    TransferSubscription,
    BlockSubscription,
    TransactionSubscription,
    MessageSubscription,
    EventSubscription,
    CandleSubscription,
    TradeSubscription,
);
