use {
    actix_web::{
        App,
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        web,
    },
    dango_httpd::{graphql::build_schema, server::config_app},
    serde::{Serialize, de::DeserializeOwned},
};

// Re-export PageInfo from indexer_testing for use in pagination helpers
pub use indexer_testing::PageInfo;

pub mod accounts;
pub mod candles;
pub mod grug;
pub mod metrics;
pub mod shutdown;
pub mod trades;
pub mod transfers;
pub mod users;

fn build_actix_app(
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

/// Helper function to paginate through all results of a GraphQL query.
///
/// This handles the pagination loop for both forward (`first`) and backward (`last`) pagination.
///
/// # Arguments
/// * `context` - The httpd context
/// * `first` - Number of items per page for forward pagination (use `Some(n)` with `last = None`)
/// * `last` - Number of items per page for backward pagination (use `Some(n)` with `first = None`)
/// * `build_query` - Closure that builds the query body given (after, before, first, last) params
/// * `extract_page` - Closure that extracts (nodes, page_info) from the response data
///
/// # Example
/// ```ignore
/// let block_heights = paginate_all(
///     dango_httpd_context.clone(),
///     Some(2), // first
///     None,    // last
///     |after, before, first, last| {
///         Accounts::build_query(accounts::Variables {
///             after,
///             before,
///             first,
///             last,
///             sort_by: Some(accounts::AccountSortBy::BLOCK_HEIGHT_DESC),
///             ..Default::default()
///         })
///     },
///     |data| {
///         let nodes = data.accounts.nodes.into_iter().map(|n| n.created_block_height).collect();
///         let page_info = PageInfo {
///             has_next_page: data.accounts.page_info.has_next_page,
///             has_previous_page: data.accounts.page_info.has_previous_page,
///             start_cursor: data.accounts.page_info.start_cursor,
///             end_cursor: data.accounts.page_info.end_cursor,
///         };
///         (nodes, page_info)
///     },
/// ).await?;
/// ```
pub async fn paginate_all<V, R, N, BuildQuery, ExtractPage>(
    context: dango_httpd::context::Context,
    first: Option<i64>,
    last: Option<i64>,
    build_query: BuildQuery,
    extract_page: ExtractPage,
) -> anyhow::Result<Vec<N>>
where
    V: Serialize,
    R: DeserializeOwned,
    BuildQuery: Fn(
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
    ) -> graphql_client::QueryBody<V>,
    ExtractPage: Fn(R) -> (Vec<N>, PageInfo),
{
    let mut all_items = vec![];
    let mut after: Option<String> = None;
    let mut before: Option<String> = None;

    loop {
        let query_body = build_query(after.clone(), before.clone(), first, last);

        let response = call_graphql_query::<V, R>(context.clone(), query_body).await?;

        let data = response.data.expect("GraphQL response should have data");
        let (nodes, page_info) = extract_page(data);

        match (first, last) {
            (Some(_), None) => {
                all_items.extend(nodes);

                if !page_info.has_next_page {
                    break;
                }
                after = page_info.end_cursor;
            },
            (None, Some(_)) => {
                // For backward pagination, reverse the nodes to maintain order
                all_items.extend(nodes.into_iter().rev());

                if !page_info.has_previous_page {
                    break;
                }
                before = page_info.start_cursor;
            },
            _ => break,
        }
    }

    Ok(all_items)
}
