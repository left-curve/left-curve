use {
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        App,
    },
    indexer_httpd::{context::Context, graphql::build_schema, server::build_actix_app},
    std::collections::HashMap,
};

// pub async fn build_actix_test_app<G>(
//     app_ctx: Context,
//     graphql_schema: G,
// ) -> App<
//     impl ServiceFactory<
//         ServiceRequest,
//         Response = ServiceResponse<impl MessageBody>,
//         Config = (),
//         InitError = (),
//         Error = Error,
//     >,
// >
// where
//     // S: async_graphql::ObjectType + Default + 'static,
//     G: Clone + 'static,
// {
//     // let graphql_schema = build_schema_with_sub::<BlankQueryHook>(app_ctx.clone());
//     // let graphql_schema = build_schema(app_ctx.clone());

//     // let app = App::new();

//     let app = actix_web::test::init_service(build_actix_app(app_ctx, graphql_schema)).await;

//     app.configure(config_app(app_ctx, graphql_schema))
// }

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
    // println!("text response: \n{:#?}", graphql_response);

    let mut graphql_response: GraphQLResponse = serde_json::from_slice(&graphql_response)?;

    // When I need to debug the response
    // println!("GraphQLResponse: {:#?}", graphql_response);

    if let Some(data) = graphql_response.data.remove(request_body.name) {
        Ok(GraphQLCustomResponse {
            data: serde_json::from_value(data)?,
            errors: graphql_response.errors,
        })
    } else {
        Err(anyhow::anyhow!(
            "Can't find {} in response",
            request_body.name
        ))
    }
}
