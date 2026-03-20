use {
    account::AccountQuery,
    async_graphql::MergedObject,
    dango_indexer_clickhouse::httpd::graphql::query::{
        candle::CandleQuery, pair_stats::PairStatsQuery, perps_candle::PerpsCandleQuery,
        trade::TradeQuery,
    },
    grug_httpd::graphql::query::grug::GrugQuery,
    indexer_httpd::graphql::query::{
        block::BlockQuery, event::EventQuery, message::MessageQuery, transaction::TransactionQuery,
    },
    perps_event::PerpsEventQuery,
    transfer::TransferQuery,
    user::UserQuery,
};

pub mod account;
pub mod perps_event;
pub mod transfer;
pub mod user;

#[derive(MergedObject, Default)]
pub struct Query(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    TransferQuery,
    GrugQuery,
    AccountQuery,
    UserQuery,
    CandleQuery,
    TradeQuery,
    PairStatsQuery,
    PerpsCandleQuery,
    PerpsEventQuery,
);
