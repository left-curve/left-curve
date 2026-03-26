use {
    account::AccountQuery, async_graphql::MergedObject,
    dango_indexer_clickhouse::httpd::graphql::query::ClickhouseQuery,
    indexer_httpd::graphql::query::IndexerQuery, perps_event::PerpsEventQuery,
    transfer::TransferQuery, user::UserQuery,
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
