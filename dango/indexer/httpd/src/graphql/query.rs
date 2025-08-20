use {
    account::AccountQuery,
    async_graphql::MergedObject,
    grug_httpd::graphql::query::grug::GrugQuery,
    indexer_clickhouse::httpd::graphql::query::{candle::CandleQuery, trade::TradeQuery},
    indexer_httpd::graphql::query::{
        block::BlockQuery, event::EventQuery, message::MessageQuery, transaction::TransactionQuery,
    },
    transfer::TransferQuery,
    user::UserQuery,
};

pub mod account;
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
);
