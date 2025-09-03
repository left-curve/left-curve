use {
    actix_codec::Framed,
    actix_http::{Request, ws},
    actix_service::IntoServiceFactory,
    actix_test::{Client, TestServer, read_body},
    actix_web::{
        App,
        body::MessageBody,
        dev::{AppConfig, ServiceFactory, ServiceRequest, ServiceResponse},
        middleware::{Compress, Logger},
        test::try_call_service,
        web::ServiceConfig,
    },
    anyhow::{anyhow, bail, ensure},
    awc::BoxedSocket,
    core::str,
    futures_util::{sink::SinkExt, stream::StreamExt},
    indexer_httpd::{context::Context, graphql::build_schema, server::config_app},
    sea_orm::sqlx::types::uuid,
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    serde_json::json,
    std::{collections::HashMap, time::Instant},
    tokio::time::{Duration, timeout},
};

pub mod block;
pub mod graphql;
pub mod setup;

const DEFAULT_TIMEOUT_SECONDS: u64 = 5;

// Re-export the configurable pagination function for use by other crates
pub use graphql::paginate_models_with_app_builder;

#[derive(Clone, Serialize, Debug)]
pub struct GraphQLCustomRequest<'a> {
    pub name: &'a str,
    pub query: &'a str,
    pub variables: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct GraphQLResponse {
    pub data: HashMap<String, serde_json::Value>,
    pub errors: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphQLSubscriptionResponse {
    pub id: String,
    pub payload: GraphQLResponse,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Serialize, Debug)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<X> {
    pub edges: Vec<Edge<X>>,
    pub nodes: Vec<X>,
    pub page_info: PageInfo,
}

#[derive(Deserialize, Serialize, Debug)]
#[allow(unused)]
pub struct Edge<X> {
    pub node: X,
    pub cursor: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[allow(unused)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

pub async fn call_batch_graphql<R, A, S, B>(
    app: A,
    requests_body: Vec<GraphQLCustomRequest<'_>>,
) -> anyhow::Result<Vec<GraphQLCustomResponse<R>>>
where
    R: DeserializeOwned,
    A: IntoServiceFactory<S, Request>,
    S: ServiceFactory<
            Request,
            Config = AppConfig,
            Response = ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    S::InitError: std::fmt::Debug,
    B: MessageBody,
{
    let app = actix_web::test::init_service(app).await;

    // When I need to debug the request body
    // println!(
    //     "request_body: {}",
    //     serde_json::to_string_pretty(&requests_body).unwrap()
    // );

    let request = actix_web::test::TestRequest::post()
        .uri("/graphql")
        .set_json(&requests_body)
        .to_request();

    let graphql_response = actix_web::test::call_and_read_body(&app, request).await;

    // When I need to debug the response
    // println!("text response: \n{graphql_response:#?}");

    let graphql_responses: Vec<GraphQLResponse> = serde_json::from_slice(&graphql_response)
        .inspect_err(|err| {
            println!("Failed to parse GraphQL response: {err}");

            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&graphql_response) {
                println!("json: {json:#?}");
            } else {
                println!("graphql_response: {graphql_response:#?}");
            }
        })?;

    // When I need to debug the response
    // println!("GraphQLResponses: {:#?}", graphql_responses);

    graphql_responses
        .into_iter()
        .enumerate()
        .map(|(index, mut graphql_response)| {
            if let Some(data) = graphql_response.data.remove(requests_body[index].name) {
                Ok(GraphQLCustomResponse {
                    data: serde_json::from_value(data)?,
                    errors: graphql_response.errors,
                })
            } else {
                bail!("can't find {} in response", requests_body[index].name)
            }
        })
        .collect::<Result<Vec<_>, _>>()
}

pub async fn call_graphql<R, A, S, B>(
    app: A,
    request_body: GraphQLCustomRequest<'_>,
) -> anyhow::Result<GraphQLCustomResponse<R>>
where
    R: DeserializeOwned,
    A: IntoServiceFactory<S, Request>,
    S: ServiceFactory<
            Request,
            Config = AppConfig,
            Response = ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    S::InitError: std::fmt::Debug,
    B: MessageBody,
{
    call_batch_graphql(app, vec![request_body])
        .await
        .and_then(|mut responses| responses.pop().ok_or_else(|| anyhow!("no response found")))
}

pub async fn call_api<R>(
    app: App<
        impl ServiceFactory<
            ServiceRequest,
            Response = ServiceResponse<impl MessageBody>,
            Config = (),
            InitError = (),
            Error = actix_web::Error,
        > + 'static,
    >,
    uri: &str,
) -> anyhow::Result<R>
where
    R: DeserializeOwned,
{
    let app = actix_web::test::init_service(app).await;

    let request = actix_web::test::TestRequest::get().uri(uri).to_request();

    let res = try_call_service(&app, request)
        .await
        .map_err(|err| anyhow!("failed to call service: {err:?}"))?;

    let text_response = read_body(res).await;

    // When I need to debug the response
    // println!("text response: \n{:#?}", str::from_utf8(&text_response)?);

    Ok(serde_json::from_slice(&text_response)?)
}

/// Calls a GraphQL subscription and returns a stream
pub async fn call_ws_graphql_stream<C, F, A, B>(
    context: C,
    app_builder: F,
    request_body: GraphQLCustomRequest<'_>,
) -> anyhow::Result<(
    TestServer,
    awc::ClientResponse,
    Framed<BoxedSocket, ws::Codec>,
)>
where
    C: Clone + Send + Sync + 'static,
    F: Fn(C) -> App<A> + Clone + Send + Sync + 'static,
    A: ServiceFactory<
            ServiceRequest,
            Response = ServiceResponse<B>,
            Config = (),
            InitError = (),
            Error = actix_web::Error,
        > + 'static,
    B: MessageBody + 'static,
{
    let srv = actix_test::start(move || app_builder(context.clone()));

    let (ws, mut framed) = Client::new()
        .ws(srv.url("/graphql"))
        .header("sec-websocket-protocol", "graphql-transport-ws")
        .connect()
        .await
        .map_err(|e| anyhow!("failed to connect to websocket 1: {e}"))?;

    framed
        .send(ws::Message::Text(
            json!({"type": "connection_init", "payload": {}})
                .to_string()
                .into(),
        ))
        .await?;

    let res = timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECONDS), framed.next()).await;

    // Wait for connection_ack
    match res {
        Ok(Some(Ok(ws::Frame::Text(text)))) => {
            ensure!(
                text == json!({ "type": "connection_ack" }).to_string(),
                "unexpected connection response: {text:?}"
            );
        },
        Ok(Some(Err(e))) => return Err(e.into()),
        Ok(None) => bail!("connection closed unexpectedly"),
        Ok(_) => bail!("unexpected message type"),
        Err(_) => bail!("connection timed out"),
    }

    let request_id = uuid::Uuid::new_v4();
    let request_body_json = json!({
        "id": request_id,
        "type": "subscribe",
        "payload": request_body
    });

    framed
        .send(ws::Message::Text(request_body_json.to_string().into()))
        .await?;

    Ok((srv, ws, framed))
}

/// Calls a GraphQL subscription and returns the first response.
pub async fn call_ws_graphql<F, A, B, R>(
    context: Context,
    app_builder: F,
    request_body: GraphQLCustomRequest<'_>,
) -> anyhow::Result<GraphQLCustomResponse<R>>
where
    R: DeserializeOwned,
    F: Fn(Context) -> App<A> + Clone + Send + Sync + 'static,
    A: ServiceFactory<
            ServiceRequest,
            Response = ServiceResponse<B>,
            Config = (),
            InitError = (),
            Error = actix_web::Error,
        > + 'static,
    B: MessageBody + 'static,
{
    let name = request_body.name;
    let (_srv, _ws, mut framed) =
        call_ws_graphql_stream(context, app_builder, request_body).await?;
    let response = parse_graphql_subscription_response(&mut framed, name).await?;

    Ok(response)
}

/// Parses a GraphQL subscription response.
pub async fn parse_graphql_subscription_response<R>(
    framed: &mut Framed<BoxedSocket, ws::Codec>,
    name: &str,
) -> anyhow::Result<GraphQLCustomResponse<R>>
where
    R: DeserializeOwned,
{
    let start = Instant::now();

    loop {
        let res = timeout(
            Duration::from_secs(DEFAULT_TIMEOUT_SECONDS - start.elapsed().as_secs()),
            framed.next(),
        )
        .await;

        match res {
            Ok(Some(Ok(ws::Frame::Text(text)))) => {
                // When I need to debug the response
                // println!("text response: \n{}", str::from_utf8(&text)?);

                let mut graphql_response: GraphQLSubscriptionResponse =
                    serde_json::from_slice(&text).inspect_err(|err| {
                        println!("Failed to parse GraphQL subscription response: {err}");

                        println!(
                            "text response: \n{}",
                            str::from_utf8(&text).unwrap_or_default()
                        );
                    })?;

                // When I need to debug the response
                // println!("response: \n{graphql_response:#?}");

                if let Some(data) = graphql_response.payload.data.remove(name) {
                    return Ok(GraphQLCustomResponse {
                        data: serde_json::from_value(data)?,
                        errors: graphql_response.payload.errors,
                    });
                } else {
                    bail!("can't find {name} in response");
                }
            },
            Ok(Some(Ok(ws::Frame::Ping(ping)))) => {
                tracing::info!("Received ping for {name}");
                framed.send(ws::Message::Pong(ping)).await?;
                continue;
            },
            Ok(Some(Err(e))) => return Err(e.into()),
            Ok(None) => bail!("connection closed unexpectedly"),
            Ok(res) => bail!("unexpected message type: {res:?}"),
            Err(_) => bail!("timeout while waiting for response for {name}"),
        }
    }
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

/// Convenience function for paginated GraphQL calls
pub async fn call_paginated_graphql<R, A, S, B>(
    app: A,
    request_body: GraphQLCustomRequest<'_>,
) -> anyhow::Result<PaginatedResponse<R>>
where
    R: DeserializeOwned,
    A: IntoServiceFactory<S, Request>,
    S: ServiceFactory<
            Request,
            Config = AppConfig,
            Response = ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    S::InitError: std::fmt::Debug,
    B: MessageBody,
{
    let response: GraphQLCustomResponse<PaginatedResponse<R>> =
        call_graphql(app, request_body).await?;

    Ok(response.data)
}
