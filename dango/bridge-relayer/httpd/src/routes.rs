use {
    actix_web::{Result, error::ErrorBadRequest, post, web},
    grug::Addr,
    std::str::FromStr,
};

#[post("/deposit-address/{dango_address}")]
async fn deposit_address(path: web::Path<String>) -> Result<String> {
    let dango_address = Addr::from_str(&path.into_inner()).map_err(ErrorBadRequest)?;

    Ok(dango_address.to_string())
}
