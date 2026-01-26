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
