use {
    account::AccountSubscription, async_graphql::MergedSubscription,
    clickhouse::ClickhouseSubscription, indexer::IndexerSubscription,
    perps_trade::PerpsTradeSubscription, transfer::TransferSubscription,
};

pub mod account;
pub mod block;
pub mod candle;
pub mod clickhouse;
pub mod event;
pub mod grug;
pub mod indexer;
pub mod message;
pub mod pair_stats;
pub mod perps_candle;
pub mod perps_pair_stats;
pub mod perps_trade;
pub mod trade;
pub mod transaction;
pub mod transfer;

pub const MAX_PAST_BLOCKS: usize = 100;

#[derive(MergedSubscription, Default)]
pub struct Subscription(
    IndexerSubscription,
    ClickhouseSubscription,
    AccountSubscription,
    TransferSubscription,
    PerpsTradeSubscription,
);
