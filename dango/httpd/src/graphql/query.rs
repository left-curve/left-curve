use {
    async_graphql::MergedObject,
    indexer_httpd::graphql::query::{
        block::BlockQuery, event::EventQuery, message::MessageQuery, tendermint::TendermintQuery,
        transaction::TransactionQuery,
    },
    transfer::TransferQuery,
};

pub mod transfer;

#[derive(MergedObject, Default)]
pub struct Query(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    TransferQuery,
    TendermintQuery,
);
