use {
    actix_web::{
        App,
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        web,
    },
    dango_httpd::{graphql::build_schema, server::config_app},
    graphql_client::GraphQLQuery,
    indexer_client::PageInfo,
    serde::{Serialize, de::DeserializeOwned},
};

// Re-export query modules from indexer_client for use in tests
pub use indexer_client::{
    Accounts, Blocks, Events, Messages, Transactions, Transfers, accounts as accounts_query,
    blocks as blocks_query, events as events_query, messages as messages_query,
    transactions as transactions_query, transfers as transfers_query,
};

mod accounts;
mod candles;
mod grug;
mod metrics;
mod shutdown;
mod trades;
mod transfers;
mod users;

pub fn build_actix_app(
    dango_httpd_context: dango_httpd::context::Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_schema(dango_httpd_context.clone());

    App::new()
        .app_data(web::Data::new(dango_httpd_context.clone()))
        .app_data(web::Data::new(
            dango_httpd_context.indexer_httpd_context.clone(),
        ))
        .app_data(web::Data::new(graphql_schema.clone()))
        .configure(config_app(dango_httpd_context, graphql_schema))
}

/// Helper function to make GraphQL queries in tests.
///
/// This reduces boilerplate by handling:
/// - Building and initializing the actix app
/// - Creating and sending the HTTP request
/// - Parsing the response
///
/// # Example
/// ```ignore
/// let response = call_graphql_query::<_, accounts::ResponseData>(
///     dango_httpd_context,
///     Accounts::build_query(accounts::Variables::default()),
/// ).await?;
/// ```
pub async fn call_graphql_query<V, R>(
    context: dango_httpd::context::Context,
    query_body: graphql_client::QueryBody<V>,
) -> anyhow::Result<graphql_client::Response<R>>
where
    V: Serialize,
    R: DeserializeOwned,
{
    let app = build_actix_app(context);
    let app = actix_web::test::init_service(app).await;

    let request = actix_web::test::TestRequest::post()
        .uri("/graphql")
        .set_json(&query_body)
        .to_request();

    let response = actix_web::test::call_and_read_body(&app, request).await;
    let response: graphql_client::Response<R> = serde_json::from_slice(&response)?;

    Ok(response)
}

/// Macro to generate pagination test helpers for GraphQL queries.
///
/// This generates a function that paginates through all results of a query
/// using the actix test context (in-process, no actual HTTP).
///
/// # Arguments
///
/// * `$fn_name` - The name of the generated function
/// * `$query_type` - The GraphQL query type (e.g., `Accounts`)
/// * `$module` - The module containing the query types (e.g., `accounts`)
/// * `$field` - The response field name (e.g., `accounts`)
/// * `$node_type` - The node type returned by the query
macro_rules! impl_test_paginate {
    ($fn_name:ident, $query_type:ty, $module:ident, $field:ident, $node_type:ident) => {
        /// Paginate through all results using the actix test context.
        ///
        /// # Arguments
        ///
        /// * `context` - The dango httpd context
        /// * `page_size` - Number of items to fetch per page
        /// * `variables` - Query variables (pagination fields will be overwritten)
        /// * `direction` - Pagination direction: `Forward` or `Backward`
        pub async fn $fn_name(
            context: dango_httpd::context::Context,
            page_size: i64,
            mut variables: $module::Variables,
            direction: PaginationDirection,
        ) -> anyhow::Result<Vec<$module::$node_type>> {
            let mut all_items = vec![];
            let mut after: Option<String> = None;
            let mut before: Option<String> = None;

            loop {
                variables.after = after.clone();
                variables.before = before.clone();

                match direction {
                    PaginationDirection::Forward => {
                        variables.first = Some(page_size);
                        variables.last = None;
                    },
                    PaginationDirection::Backward => {
                        variables.first = None;
                        variables.last = Some(page_size);
                    },
                }

                let query_body = <$query_type>::build_query(variables.clone());
                let response = call_graphql_query::<_, $module::ResponseData>(
                    context.clone(),
                    query_body,
                )
                .await?;

                let data = response.data.expect("GraphQL response should have data");
                let connection = data.$field;
                let page_info = PageInfo {
                    start_cursor: connection.page_info.start_cursor,
                    end_cursor: connection.page_info.end_cursor,
                    has_next_page: connection.page_info.has_next_page,
                    has_previous_page: connection.page_info.has_previous_page,
                };

                match direction {
                    PaginationDirection::Forward => {
                        all_items.extend(connection.nodes);
                        if !page_info.has_next_page {
                            break;
                        }
                        after = page_info.end_cursor;
                    },
                    PaginationDirection::Backward => {
                        all_items.extend(connection.nodes.into_iter().rev());
                        if !page_info.has_previous_page {
                            break;
                        }
                        before = page_info.start_cursor;
                    },
                }
            }

            Ok(all_items)
        }
    };
}

/// Direction for pagination.
#[derive(Clone, Copy)]
pub enum PaginationDirection {
    /// Paginate forward using `first` and `after`
    Forward,
    /// Paginate backward using `last` and `before`
    Backward,
}

// Generate pagination test helpers for all paginated query types
impl_test_paginate!(
    paginate_accounts,
    Accounts,
    accounts_query,
    accounts,
    AccountsAccountsNodes
);
impl_test_paginate!(
    paginate_transfers,
    Transfers,
    transfers_query,
    transfers,
    TransfersTransfersNodes
);
impl_test_paginate!(
    paginate_transactions,
    Transactions,
    transactions_query,
    transactions,
    TransactionsTransactionsNodes
);
impl_test_paginate!(
    paginate_blocks,
    Blocks,
    blocks_query,
    blocks,
    BlocksBlocksNodes
);
impl_test_paginate!(
    paginate_events,
    Events,
    events_query,
    events,
    EventsEventsNodes
);
impl_test_paginate!(
    paginate_messages,
    Messages,
    messages_query,
    messages,
    MessagesMessagesNodes
);
