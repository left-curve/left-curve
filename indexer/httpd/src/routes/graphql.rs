use {
    crate::graphql::AppSchema,
    actix_web::{HttpRequest, HttpResponse, Resource, web},
    async_graphql::{Schema, http::*},
    async_graphql_actix_web::{GraphQLRequest, GraphQLResponse, GraphQLSubscription},
};

pub fn graphql_route() -> Resource {
    web::resource("/graphql")
        .route(web::post().to(graphql_index))
        .route(
            web::get()
                .guard(actix_web::guard::Header("upgrade", "websocket"))
                .to(graphql_ws),
        )
        .route(web::get().to(graphiql_playgound))
}

pub(crate) async fn graphql_index(
    schema: web::Data<AppSchema>,
    _req: HttpRequest,
    gql_request: GraphQLRequest,
) -> GraphQLResponse {
    let request = gql_request.into_inner();

    schema.execute(request).await.into()
}

pub async fn graphiql_playgound() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            GraphiQLSource::build()
                .endpoint("/graphql")
                .subscription_endpoint("/graphql")
                // .credentials(Credentials::Include)
                .finish(),
        )
}

pub(crate) async fn graphql_ws(
    schema: web::Data<AppSchema>,
    req: HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse> {
    GraphQLSubscription::new(Schema::clone(&*schema)).start(&req, payload)
}
