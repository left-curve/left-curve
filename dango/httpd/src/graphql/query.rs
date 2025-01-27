use {
    async_graphql::MergedObject,
    indexer_httpd::graphql::query::{
        block::BlockQuery, event::EventQuery, message::MessageQuery, transaction::TransactionQuery,
    },
    transfer::TransferQuery,
};

pub mod transfer;

#[derive(MergedObject, Default)]
pub struct Query(
    TransferQuery,
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
);
