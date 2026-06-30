use {
    crate::{
        context::{FullContext, MinimalContext},
        graphql::query::core::CoreQuery,
        request_ip::RequesterIp,
    },
    actix_web::{
        Error, HttpRequest, HttpResponse, Scope,
        error::{ErrorBadRequest, ErrorInternalServerError},
        post, web,
    },
    dango_primitives::{Query, Tx, UnsignedTx},
};

/// REST endpoints that mirror the GraphQL `queryApp` / `simulate` queries and
/// the `broadcastTxSync` mutation. Each takes the raw object as its JSON body
/// (no wrapper) and returns the raw response JSON (no GraphQL envelope).
pub fn services() -> Scope {
    // These are top-level paths with no shared prefix, so the scope prefix is
    // empty; the per-handler `#[post("/...")]` paths are absolute.
    web::scope("")
        .service(query)
        .service(simulate)
        .service(broadcast)
}

/// Run a read-only query against the latest finalized state.
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

/// Dry-run an unsigned transaction, returning its simulated outcome.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[post("/simulate")]
pub async fn simulate(
    body: web::Json<UnsignedTx>,
    app_ctx: web::Data<MinimalContext>,
) -> Result<HttpResponse, Error> {
    let outcome = CoreQuery::_simulate(&app_ctx, body.into_inner())
        .await
        .map_err(|e| ErrorBadRequest(e.message))?;

    Ok(HttpResponse::Ok().json(outcome))
}

/// Broadcast a signed transaction to the mempool. Returns a mempool receipt
/// (`BroadcastTxOutcome`), not block inclusion; a mempool-rejected tx still
/// returns `200` with a non-zero `check_tx.code`. Only a transport failure to
/// the consensus client returns `500`.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
#[post("/broadcast")]
pub async fn broadcast(
    body: web::Json<Tx>,
    app_ctx: web::Data<FullContext>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let details = RequesterIp::from_request(&req).into_http_request_details();

    let outcome = crate::broadcast::broadcast_tx(&app_ctx, &details, body.into_inner())
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(outcome))
}
