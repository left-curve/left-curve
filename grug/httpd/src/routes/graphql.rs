use {
    crate::request_ip::RequesterIp,
    actix_web::{HttpRequest, HttpResponse, Resource, http::header, web},
    async_graphql::{Schema, http::GraphiQLSource},
    async_graphql_actix_web::{GraphQLBatchRequest, GraphQLResponse, GraphQLSubscription},
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

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
    let requester_ip = RequesterIp::from_request(&req);
    let details = requester_ip.clone().into_http_request_details();

    let request = gql_request.into_inner().data(details).data(requester_ip);

    schema.execute_batch(request).await.into()
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
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
