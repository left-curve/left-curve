use {
    account::AccountQuery, async_graphql::MergedObject, block::BlockQuery,
    clickhouse::ClickhouseQuery, core::CoreQuery, event::EventQuery, message::MessageQuery,
    perps_event::PerpsEventQuery, transaction::TransactionQuery, transfer::TransferQuery,
    user::UserQuery,
};

pub mod account;
pub mod block;
pub mod clickhouse;
pub mod core;
pub mod event;
pub mod message;
pub mod pagination;
pub mod perps_event;
pub mod transaction;
pub mod transfer;
pub mod user;

#[derive(MergedObject, Default)]
pub struct IndexerQuery(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    CoreQuery,
);

#[derive(MergedObject, Default)]
#[graphql(name = "Query")]
pub struct FullQuery(
    IndexerQuery,
    ClickhouseQuery,
    TransferQuery,
    AccountQuery,
    UserQuery,
    PerpsEventQuery,
);
