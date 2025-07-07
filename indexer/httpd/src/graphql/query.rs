use {
    async_graphql::MergedObject, block::BlockQuery, event::EventQuery,
    grug_httpd::graphql::query::grug::GrugQuery, message::MessageQuery,
    transaction::TransactionQuery,
};

pub mod block;
pub mod event;
pub mod message;
pub mod pagination;
pub mod transaction;

#[derive(MergedObject, Default)]
pub struct Query(
    BlockQuery,
    TransactionQuery,
    MessageQuery,
    EventQuery,
    GrugQuery,
);
