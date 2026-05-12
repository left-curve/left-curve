use {
    account::AccountQuery, async_graphql::MergedObject, clickhouse::ClickhouseQuery,
    indexer::IndexerQuery, perps_event::PerpsEventQuery, transfer::TransferQuery, user::UserQuery,
};

pub mod account;
pub mod block;
pub mod candle;
pub mod clickhouse;
pub mod event;
pub mod grug;
pub mod indexer;
pub mod message;
pub mod pagination;
pub mod pair_stats;
pub mod perps_candle;
pub mod perps_event;
pub mod perps_fees;
pub mod perps_pair_stats;
pub mod trade;
pub mod transaction;
pub mod transfer;
pub mod user;

#[derive(MergedObject, Default)]
pub struct Query(
    IndexerQuery,
    ClickhouseQuery,
    TransferQuery,
    AccountQuery,
    UserQuery,
    PerpsEventQuery,
);
