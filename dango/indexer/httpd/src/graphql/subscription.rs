use {
    account::AccountSubscription,
    async_graphql::*,
    dango_indexer_clickhouse::httpd::graphql::subscription::{
        candle::CandleSubscription, perps_candle::PerpsCandleSubscription, trade::TradeSubscription,
    },
    indexer_httpd::graphql::subscription::{
        block::BlockSubscription, event::EventSubscription, grug::GrugSubscription,
        message::MessageSubscription, transaction::TransactionSubscription,
    },
    perps_trade::PerpsTradeSubscription,
    transfer::TransferSubscription,
};

pub mod account;
pub mod perps_trade;
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
    PerpsCandleSubscription,
    PerpsTradeSubscription,
    GrugSubscription,
);
