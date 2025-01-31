use {
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        middleware::{Compress, Logger},
        web::ServiceConfig,
        App,
    },
    anyhow::anyhow,
    indexer_httpd::{context::Context, graphql::build_schema, server::config_app},
    serde::Deserialize,
    std::collections::HashMap,
};

#[derive(serde::Serialize, Debug)]
pub struct GraphQLCustomRequest<'a> {
    pub name: &'a str,
    pub query: &'a str,
    pub variables: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Deserialize, Debug)]
pub struct GraphQLResponse {
    pub data: HashMap<String, serde_json::Value>,
    pub errors: Option<Vec<serde_json::Value>>,
}

#[derive(serde::Deserialize, Debug)]
pub struct GraphQLCustomResponse<R> {
    pub data: R,
    pub errors: Option<Vec<serde_json::Value>>,
}

pub fn build_app_service(
    app_ctx: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_schema(app_ctx.clone());

    build_actix_app(app_ctx, graphql_schema)
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<X> {
    pub edges: Vec<Edge<X>>,
    pub nodes: Vec<X>,
    pub page_info: PageInfo,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct Edge<X> {
    pub node: X,
    pub cursor: String,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub start_cursor: String,
    pub end_cursor: String,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub async fn call_graphql<R>(
    app: App<
        impl ServiceFactory<
                ServiceRequest,
                Response = ServiceResponse<impl MessageBody>,
                Config = (),
                InitError = (),
                Error = actix_web::Error,
            > + 'static,
    >,
    request_body: GraphQLCustomRequest<'_>,
) -> Result<GraphQLCustomResponse<R>, anyhow::Error>
where
    R: serde::de::DeserializeOwned,
{
    let app = actix_web::test::init_service(app).await;

    let request = actix_web::test::TestRequest::post()
        .uri("/graphql")
        .set_json(&request_body)
        .to_request();

    let graphql_response = actix_web::test::call_and_read_body(&app, request).await;

    // When I need to debug the response
    println!("text response: \n{:#?}", graphql_response);

    let mut graphql_response: GraphQLResponse = serde_json::from_slice(&graphql_response)?;

    // When I need to debug the response
    println!("GraphQLResponse: {:#?}", graphql_response);

    if let Some(data) = graphql_response.data.remove(request_body.name) {
        Ok(GraphQLCustomResponse {
            data: serde_json::from_value(data)?,
            errors: graphql_response.errors,
        })
    } else {
        Err(anyhow!("can't find {} in response", request_body.name))
    }
}

use {
    actix_http::ws,
    actix_web::web::Bytes,
    futures_util::{SinkExt as _, StreamExt as _},
};

pub async fn call_ws_graphql<R>(
    app: App<
        impl ServiceFactory<
                ServiceRequest,
                Response = ServiceResponse<impl MessageBody>,
                Config = (),
                InitError = (),
                Error = actix_web::Error,
            > + 'static,
    >,
    request_body: GraphQLCustomRequest<'_>,
) -> Result<GraphQLCustomResponse<R>, anyhow::Error>
where
    R: serde::de::DeserializeOwned,
{
    // let mut srv = actix_test::start(|| app);

    todo!()
    // let app = actix_web::test::init_service(app).await;

    // let request = actix_web::test::TestRequest::post()
    //     .uri("/graphql")
    //     .set_json(&request_body)
    //     .to_request();

    // let graphql_response = actix_web::test::ws_connect(&app, request).await;

    // // When I need to debug the response
    // println!("text response: \n{:#?}", graphql_response);

    // let mut graphql_response: GraphQLResponse = serde_json::from_slice(&graphql_response)?;

    // // When I need to debug the response
    // println!("GraphQLResponse: {:#?}", graphql_response);

    // if let Some(data) = graphql_response.data.remove(request_body.name) {
    //     Ok(GraphQLCustomResponse {
    //         data: serde_json::from_value(data)?,
    //         errors: graphql_response.errors,
    //     })
    // } else {
    //     Err(anyhow!("can't find {} in response", request_body.name))
    // }
}

pub fn build_actix_app<G>(
    app_ctx: Context,
    graphql_schema: G,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
>
where
    G: Clone + 'static,
{
    build_actix_app_with_config(app_ctx, graphql_schema, |app_ctx, graphql_schema| {
        config_app(app_ctx, graphql_schema)
    })
}

/// Builds an Actix app with a custom config function. Used for Dango to have
/// a different GraphQL executor and custom routes to use that executor.
///
/// I tried really hard to use async-graphql + generics but couldn't get it to
/// work. I'm not sure that's doable.
///
/// See <https://github.com/async-graphql/async-graphql/discussions/1630>.
pub fn build_actix_app_with_config<F, G>(
    app_ctx: Context,
    graphql_schema: G,
    config_app: F,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
>
where
    G: Clone + 'static,
    F: FnOnce(Context, G) -> Box<dyn Fn(&mut ServiceConfig)>,
{
    let app = App::new().wrap(Logger::default()).wrap(Compress::default());

    app.configure(config_app(app_ctx, graphql_schema))
}
