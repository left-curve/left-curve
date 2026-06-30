use {
    crate::{context::MinimalContext, graphql::query::core::CoreQuery},
    actix_web::{Error, HttpResponse, error::ErrorBadRequest, post, web},
    dango_primitives::UnsignedTx,
};

/// `POST /simulate` — dry-run an `UnsignedTx`, returning its simulated
/// `TxOutcome`. Mirrors the GraphQL `simulate` query.
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
