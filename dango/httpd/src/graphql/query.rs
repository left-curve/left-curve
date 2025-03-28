use {
    account::AccountQuery,
    async_graphql::MergedObject,
    indexer_httpd::graphql::query::{
        block::BlockQuery, event::EventQuery, grug::GrugQuery, message::MessageQuery,
        tendermint::TendermintQuery, transaction::TransactionQuery,
    },
    transfer::TransferQuery,
};

pub mod account;
pub mod transfer;

#[derive(MergedObject, Default)]
pub struct Query(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    TransferQuery,
    TendermintQuery,
    GrugQuery,
    AccountQuery,
);
