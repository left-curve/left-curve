use {
    account::AccountQuery,
    async_graphql::MergedObject,
    indexer_httpd::graphql::query::{
        block::BlockQuery, event::EventQuery, grug::GrugQuery, message::MessageQuery,
        transaction::TransactionQuery,
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
);
