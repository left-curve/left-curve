use {
    actix_web::{Error, HttpResponse, error::ErrorInternalServerError, get, web},
    std::str::FromStr,
    tendermint_rpc::Client,
};

use crate::context::Context;

#[get("/search_tx/{hash}")]
pub async fn search_tx(
    path: web::Path<u64>,
    app_ctx: web::Data<Context>,
) -> Result<HttpResponse, Error> {
    let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())
        .map_err(ErrorInternalServerError)?;

    let tx_hash = tendermint::hash::Hash::from_str(&path.into_inner().to_string())
        .map_err(ErrorInternalServerError)?;

    let tx = http_client
        .tx(tx_hash, false)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(tx))
}
