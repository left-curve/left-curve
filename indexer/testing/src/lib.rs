use {
    actix_codec::Framed,
    actix_http::ws,
    actix_test::{read_body, Client, TestServer},
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        middleware::{Compress, Logger},
        test::try_call_service,
        web::ServiceConfig,
        App,
    },
    anyhow::{anyhow, bail, ensure},
    awc::BoxedSocket,
    core::str,
    futures_util::{sink::SinkExt, stream::StreamExt},
    indexer_httpd::{context::Context, graphql::build_schema, server::config_app},
    sea_orm::sqlx::types::uuid,
    serde::{de::DeserializeOwned, Deserialize, Serialize},
    serde_json::json,
    std::collections::HashMap,
};

#[derive(Serialize, Debug)]
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
) -> anyhow::Result<GraphQLCustomResponse<R>>
where
    R: DeserializeOwned,
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
        Err(anyhow!("can't find {} in response", request_body.name))
    }
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
        .map_err(|_err| anyhow!("Failed to call service"))?;
    let text_response = read_body(res).await;

    Ok(serde_json::from_slice(&text_response)?)
}

/// Calls a GraphQL subscription and returns a stream
pub async fn call_ws_graphql_stream<F, A, B>(
    context: Context,
    app_builder: F,
    request_body: GraphQLCustomRequest<'_>,
) -> anyhow::Result<(
    TestServer,
    awc::ClientResponse,
    Framed<BoxedSocket, ws::Codec>,
)>
where
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

    // Wait for connection_ack
    match framed.next().await {
        Some(Ok(ws::Frame::Text(text))) => {
            ensure!(
                text == json!({ "type": "connection_ack" }).to_string(),
                "unexpected connection response: {text:?}"
            );
        },
        Some(Err(e)) => return Err(e.into()),
        None => bail!("connection closed unexpectedly"),
        _ => bail!("unexpected message type"),
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
    let (_srv, _ws, framed) = call_ws_graphql_stream(context, app_builder, request_body).await?;
    let (_, response) = parse_graphql_subscription_response(framed, name).await?;
    Ok(response)
}

/// Parses a GraphQL subscription response.
pub async fn parse_graphql_subscription_response<R>(
    mut framed: Framed<BoxedSocket, ws::Codec>,
    name: &str,
) -> anyhow::Result<(Framed<BoxedSocket, ws::Codec>, GraphQLCustomResponse<R>)>
where
    R: DeserializeOwned,
{
    match framed.next().await {
        Some(Ok(ws::Frame::Text(text))) => {
            // When I need to debug the response
            // println!("text response: \n{}", str::from_utf8(&text)?);

            let mut graphql_response: GraphQLSubscriptionResponse = serde_json::from_slice(&text)?;

            // When I need to debug the response
            // println!("response: \n{:#?}", graphql_response);

            if let Some(data) = graphql_response.payload.data.remove(name) {
                Ok((framed, GraphQLCustomResponse {
                    data: serde_json::from_value(data)?,
                    errors: graphql_response.payload.errors,
                }))
            } else {
                Err(anyhow!("can't find {name} in response"))
            }
        },
        Some(Ok(ws::Frame::Ping(ping))) => {
            framed.send(ws::Message::Pong(ping)).await?;
            Err(anyhow!("received ping"))
        },
        Some(Err(e)) => Err(e.into()),
        None => Err(anyhow!("connection closed unexpectedly")),
        res => Err(anyhow!("unexpected message type: {res:?}")),
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
