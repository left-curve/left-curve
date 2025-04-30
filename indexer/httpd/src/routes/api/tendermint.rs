use {
    actix_web::{Error, HttpResponse, error::ErrorInternalServerError, get, web},
    grug_types::Hash256,
    std::str::FromStr,
    tracing::info,
};

use crate::context::Context;

#[get("/search_tx/{hash}")]
pub async fn search_tx(
    path: web::Path<String>,
    app_ctx: web::Data<Context>,
) -> Result<HttpResponse, Error> {
    let tx_hash = Hash256::from_str(&path.into_inner()).map_err(ErrorInternalServerError)?;

    let tx = app_ctx
        .consensus_client
        .search_tx(tx_hash)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(tx))
}
