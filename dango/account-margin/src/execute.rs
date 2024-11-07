use {
    anyhow::ensure,
    dango_auth::authenticate_tx,
    dango_types::{
        account::{InstantiateMsg, Tx},
        config::ACCOUNT_FACTORY_KEY,
    },
    grug::{Addr, AuthCtx, AuthResponse, MutableCtx, Response, StdResult},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    let account_factory: Addr = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;

    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == account_factory,
        "you don't have the right, O you don't have the right"
    );

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    authenticate_tx(ctx, tx, None)?;

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    // Do nothing, accept all transfers.
    Ok(Response::new())
}
