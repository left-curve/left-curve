use {
    crate::context::Context,
    actix_web::{Result, error::ErrorBadRequest, post, web},
    dango_types::bitcoin::MultisigWallet,
    grug::Addr,
    std::str::FromStr,
};

#[post("/deposit-address/{dango_address}")]
async fn deposit_address(path: web::Path<String>, context: web::Data<Context>) -> Result<String> {
    let _dango_address = Addr::from_str(&path.into_inner()).map_err(ErrorBadRequest)?;

    // TODO: Pass in dango address once MultisigWallet is updated to accept it instead of index.
    let multisig_wallet = MultisigWallet::new(&context.multisig_settings, None);

    let bitcoin_deposit_address = multisig_wallet.address(context.network);

    // TODO: Store the deposit address in the database.

    Ok(bitcoin_deposit_address.to_string())
}
