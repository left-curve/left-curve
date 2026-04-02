use {
    account::AccountSubscription, async_graphql::MergedSubscription,
    dango_indexer_clickhouse::httpd::graphql::subscription::ClickhouseSubscription,
    indexer_httpd::graphql::subscription::IndexerSubscription, perps_trade::PerpsTradeSubscription,
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
