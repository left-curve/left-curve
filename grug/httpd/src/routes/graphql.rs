use {
    actix_web::{HttpRequest, HttpResponse, Resource, http::header, web},
    async_graphql::{Schema, http::GraphiQLSource},
    async_graphql_actix_web::{GraphQLBatchRequest, GraphQLResponse, GraphQLSubscription},
    grug_types::HttpRequestDetails,
};

pub fn graphql_route<Q, M, S>() -> Resource
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    web::resource("/graphql")
        .route(web::post().to(graphql_index::<Q, M, S>))
        .route(
            web::get()
                .guard(actix_web::guard::Header("upgrade", "websocket"))
                .to(graphql_ws::<Q, M, S>),
        )
        .route(web::get().to(graphiql_playground))
}

pub async fn graphql_index<Q, M, S>(
    schema: web::Data<Schema<Q, M, S>>,
    req: HttpRequest,
    gql_request: GraphQLBatchRequest,
) -> GraphQLResponse
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    let remote_ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|ip| ip.to_string());

    let peer_ip = req.connection_info().peer_addr().map(|ip| ip.to_string());

    let details = HttpRequestDetails::new(remote_ip, peer_ip);

    let request = gql_request.into_inner().data(details);

    schema.execute_batch(request).await.into()
}

pub async fn graphiql_playground() -> HttpResponse {
    let html = GraphiQLSource::build()
        .endpoint("/graphql")
        .subscription_endpoint("/graphql")
        .finish();

    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/html; charset=utf-8"))
        .insert_header((
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; script-src 'self' 'unsafe-eval'",
        ))
        .body(html)
}

pub async fn graphql_ws<Q, M, S>(
    schema: web::Data<Schema<Q, M, S>>,
    req: HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse>
where
    Q: async_graphql::ObjectType + 'static,
    M: async_graphql::ObjectType + 'static,
    S: async_graphql::SubscriptionType + 'static,
{
    GraphQLSubscription::new(Schema::clone(&*schema)).start(&req, payload)
}
