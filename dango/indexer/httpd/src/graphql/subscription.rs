use {
    account::AccountSubscription,
    async_graphql::MergedSubscription,
    indexer_httpd::graphql::subscription::{
        IndexerSubscription, clickhouse::ClickhouseSubscription,
    },
    perps_trade::PerpsTradeSubscription,
    transfer::TransferSubscription,
};

pub mod account;
pub mod perps_trade;
pub mod transfer;

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    IndexerSubscription,
    ClickhouseSubscription,
    AccountSubscription,
    TransferSubscription,
    PerpsTradeSubscription,
);
