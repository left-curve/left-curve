use {
    crate::graphql::AppSchema,
    actix_web::{web, HttpRequest, HttpResponse},
    async_graphql::{http::*, Schema},
    async_graphql_actix_web::{GraphQLRequest, GraphQLResponse, GraphQLSubscription},
};

#[tracing::instrument(name = "graphql::graphql_index", skip_all)]
pub(crate) async fn graphql_index(
    schema: web::Data<AppSchema>,
    _req: HttpRequest,
    gql_request: GraphQLRequest,
) -> GraphQLResponse {
    let request = gql_request.into_inner();

    schema.execute(request).await.into()
}

#[tracing::instrument(name = "graphql::graphiql_playgound")]
pub(crate) async fn graphiql_playgound() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(
            GraphiQLSource::build()
                .endpoint("/graphql")
                .credentials(Credentials::Include)
                .finish(),
        )
}

#[tracing::instrument(name = "graphql::graphql_ws", skip_all)]
pub(crate) async fn graphql_ws(
    schema: web::Data<AppSchema>,
    req: HttpRequest,
    payload: web::Payload,
) -> actix_web::Result<HttpResponse> {
    GraphQLSubscription::new(Schema::clone(&*schema)).start(&req, payload)
}
