use {
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        web::{self, ServiceConfig},
        App, Error,
    },
    indexer_httpd::{context::Context, graphql::build_schema, routes, server::build_actix_app},
};

pub async fn build_actix_test_app(
    app_ctx: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = Error,
    >,
> {
    let context = Context::new(None).await.expect("Can't create context");
    let graphql_schema = build_schema(context.clone());

    let app = App::new()
        .service(routes::index::index)
        .service(routes::graphql::graphql_route())
        .app_data(web::Data::new(app_ctx.clone()))
        .app_data(web::Data::new(graphql_schema.clone()));

    app.configure(config_app())
}

pub fn config_app(// app_ctx: web::Data<AppContext>,
    // schema: web::Data<AppSchema>,
) -> Box<dyn Fn(&mut ServiceConfig)> {
    todo!()
}

#[derive(serde::Serialize, Debug)]
pub struct GraphQLCustomRequest<'a> {
    pub query: &'a str,
    pub variables: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Deserialize, Debug)]
pub struct GraphQLCustomResponse {
    pub data: serde_json::Value,
    pub errors: Option<Vec<serde_json::Value>>,
}

pub async fn call_graphql(
    app_ctx: Context,
    request_body: GraphQLCustomRequest<'_>,
) -> Result<GraphQLCustomResponse, anyhow::Error> {
    let app = actix_web::test::init_service(build_actix_app(app_ctx)).await;

    let request = actix_web::test::TestRequest::post()
        .uri("/graphql")
        .set_json(&request_body)
        .to_request();

    Ok(actix_web::test::call_and_read_body_json(&app, request).await)
}
