use {
    actix_web::{
        App,
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        web,
    },
    indexer_httpd::{
        graphql::build_full_schema, routes::graphql::GraphqlRequestTimeout, server::config_app,
        subscription_limiter::SubscriptionLimiter,
    },
    serde::{Serialize, de::DeserializeOwned},
    std::time::Duration,
};

// Re-export PaginationDirection from indexer_testing
pub use indexer_testing::PaginationDirection;

// Re-export query modules from indexer_graphql_types for use in tests
pub use indexer_graphql_types::{
    Accounts, Blocks, Events, Messages, Transactions, Transfers, accounts as accounts_query,
    blocks as blocks_query, events as events_query, messages as messages_query,
    transactions as transactions_query, transfers as transfers_query,
};

mod accounts;
mod candles;
mod metrics;
mod pair_stats;
mod perps_candles;
mod perps_events;
mod perps_pair_stats;
mod shutdown;
mod trades;
mod transfers;
mod users;

pub fn build_actix_app(
    dango_httpd_context: indexer_httpd::context::FullContext,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_full_schema(dango_httpd_context.clone());

    App::new()
        .app_data(web::Data::new(SubscriptionLimiter::new(10, 5000)))
        .app_data(web::Data::new(GraphqlRequestTimeout(Duration::from_secs(
            30,
        ))))
        .app_data(web::Data::new(dango_httpd_context.clone()))
        .app_data(web::Data::new(dango_httpd_context.clone()))
        .app_data(web::Data::new(graphql_schema.clone()))
        .configure(config_app(dango_httpd_context, graphql_schema))
}

/// Helper function to make GraphQL queries in tests.
///
/// This reduces boilerplate by handling:
/// - Building the dango actix app from context
/// - Delegating to indexer_testing::call_graphql_query
///
/// # Example
/// ```ignore
/// let response = call_graphql_query::<_, accounts::ResponseData>(
///     dango_httpd_context,
///     Accounts::build_query(accounts::Variables::default()),
/// ).await?;
/// ```
pub async fn call_graphql_query<V, R>(
    context: indexer_httpd::context::FullContext,
    query_body: graphql_client::QueryBody<V>,
) -> anyhow::Result<graphql_client::Response<R>>
where
    V: Serialize,
    R: DeserializeOwned,
{
    let app = build_actix_app(context);
    indexer_testing::call_graphql_query(app, query_body).await
}

// Generate pagination test helpers using the shared macro from indexer_testing
indexer_testing::impl_paginate!(
    paginate_accounts,
    indexer_httpd::context::FullContext,
    Accounts,
    accounts_query,
    accounts,
    AccountsAccountsNodes,
    build_actix_app
);
indexer_testing::impl_paginate!(
    paginate_transfers,
    indexer_httpd::context::FullContext,
    Transfers,
    transfers_query,
    transfers,
    TransfersTransfersNodes,
    build_actix_app
);
indexer_testing::impl_paginate!(
    paginate_transactions,
    indexer_httpd::context::FullContext,
    Transactions,
    transactions_query,
    transactions,
    TransactionsTransactionsNodes,
    build_actix_app
);
indexer_testing::impl_paginate!(
    paginate_blocks,
    indexer_httpd::context::FullContext,
    Blocks,
    blocks_query,
    blocks,
    BlocksBlocksNodes,
    build_actix_app
);
indexer_testing::impl_paginate!(
    paginate_events,
    indexer_httpd::context::FullContext,
    Events,
    events_query,
    events,
    EventsEventsNodes,
    build_actix_app
);
indexer_testing::impl_paginate!(
    paginate_messages,
    indexer_httpd::context::FullContext,
    Messages,
    messages_query,
    messages,
    MessagesMessagesNodes,
    build_actix_app
);
