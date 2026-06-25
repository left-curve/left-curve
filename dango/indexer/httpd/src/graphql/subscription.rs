use {
    account::AccountSubscription, async_graphql::*, block::BlockSubscription,
    clickhouse::ClickhouseSubscription, core::CoreSubscription, event::EventSubscription,
    full_block::FullBlockSubscription, message::MessageSubscription,
    perps_events2::PerpsEvents2Subscription, perps_trade::PerpsTradeSubscription,
    transaction::TransactionSubscription, transfer::TransferSubscription,
};

pub mod account;
pub mod block;
pub mod clickhouse;
pub mod core;
pub mod event;
pub mod full_block;
pub mod message;
pub mod perps_events2;
pub mod perps_trade;
pub mod transaction;
pub mod transfer;

pub const MAX_PAST_BLOCKS: usize = 100;

#[derive(MergedSubscription, Default)]
pub struct IndexerSubscription(
    BlockSubscription,
    TransactionSubscription,
    MessageSubscription,
    EventSubscription,
    CoreSubscription,
);

#[derive(MergedSubscription, Default)]
#[graphql(name = "Subscription")]
pub struct FullSubscription(
    IndexerSubscription,
    ClickhouseSubscription,
    AccountSubscription,
    TransferSubscription,
    PerpsTradeSubscription,
    PerpsEvents2Subscription,
    FullBlockSubscription,
);
