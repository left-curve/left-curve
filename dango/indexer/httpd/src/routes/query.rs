use {
    crate::{context::MinimalContext, graphql::query::core::CoreQuery},
    actix_web::{Error, HttpResponse, error::ErrorBadRequest, post, web},
    dango_primitives::Query,
};

/// `POST /query` — run a read-only query against the latest finalized state.
/// The body is a raw `Query` object; the response is the raw `QueryResponse`
/// (no GraphQL envelope). Mirrors the GraphQL `queryApp` query.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[post("/query")]
pub async fn query(
    body: web::Json<Query>,
    app_ctx: web::Data<MinimalContext>,
) -> Result<HttpResponse, Error> {
    let response = CoreQuery::_query_app(&app_ctx, body.into_inner())
        .await
        .map_err(|e| ErrorBadRequest(e.message))?
        .response;

    Ok(HttpResponse::Ok().json(response))
}
