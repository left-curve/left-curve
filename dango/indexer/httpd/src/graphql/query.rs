use {
    account::AccountQuery,
    async_graphql::MergedObject,
    indexer_httpd::graphql::query::{IndexerQuery, clickhouse::ClickhouseQuery},
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
    IndexerQuery,
    ClickhouseQuery,
    TransferQuery,
    AccountQuery,
    UserQuery,
    PerpsEventQuery,
);
