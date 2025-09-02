use {
    crate::graphql::AppSchema,
    actix_web::{HttpRequest, HttpResponse, Resource, http::header, web},
    async_graphql::{Schema, http::GraphiQLSource},
    async_graphql_actix_web::{GraphQLBatchRequest, GraphQLResponse, GraphQLSubscription},
    grug_app::HttpRequestDetails,
};

pub fn graphql_route() -> Resource {
    web::resource("/graphql")
        .route(web::post().to(graphql_index))
        .route(
            web::get()
                .guard(actix_web::guard::Header("upgrade", "websocket"))
                .to(graphql_ws),
        )
        .route(web::get().to(graphiql_playground))
}

pub(crate) async fn graphql_index(
    schema: web::Data<AppSchema>,
    req: HttpRequest,
    gql_request: GraphQLBatchRequest,
) -> GraphQLResponse {
    let remote_ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|ip| ip.to_string());

    let peer_ip = req.connection_info().peer_addr().map(|ip| ip.to_string());

    let details = HttpRequestDetails { remote_ip, peer_ip };

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

pub(crate) async fn graphql_ws(
    schema: web::Data<AppSchema>,
    req: HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse> {
    GraphQLSubscription::new(Schema::clone(&*schema)).start(&req, payload)
}
