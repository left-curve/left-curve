use {
    account::AccountSubscription, async_graphql::*, block::BlockSubscription,
    clickhouse::ClickhouseSubscription, event::EventSubscription, grug::GrugSubscription,
    message::MessageSubscription, perps_trade::PerpsTradeSubscription,
    transaction::TransactionSubscription, transfer::TransferSubscription,
};

pub mod account;
pub mod block;
pub mod clickhouse;
pub mod event;
pub mod grug;
pub mod message;
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
    GrugSubscription,
);

#[derive(MergedSubscription, Default)]
#[graphql(name = "Subscription")]
pub struct FullSubscription(
    IndexerSubscription,
    ClickhouseSubscription,
    AccountSubscription,
    TransferSubscription,
    PerpsTradeSubscription,
);
