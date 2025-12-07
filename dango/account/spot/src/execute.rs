use {
    dango_types::account::spot::InstantiateMsg,
    grug::{AuthCtx, AuthResponse, MutableCtx, Response, Tx},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    dango_auth::create_account(ctx)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    dango_auth::authenticate_tx(ctx, tx, None)?;

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(ctx: MutableCtx) -> anyhow::Result<Response> {
    dango_auth::receive_transfer(ctx)?;

    Ok(Response::new())
}
