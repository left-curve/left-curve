use {
    anyhow::ensure,
    dango_auth::{authenticate_tx, receive_transfer},
    dango_types::{DangoQuerier, account::spot::InstantiateMsg},
    grug::{AuthCtx, AuthResponse, MutableCtx, Response, Tx},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == ctx.querier.query_account_factory()?,
        "you don't have the right, O you don't have the right"
    );

    // Upon creation, the account's status is set to `Inactive`.
    // We don't need to save it in storage, because if storage is empty, it's
    // default to `Inactive`. This is an intentional optimization to minimize
    // disk writes.

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    authenticate_tx(ctx, tx, None)?;

    Ok(AuthResponse::new().request_backrun(false))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(ctx: MutableCtx) -> anyhow::Result<Response> {
    receive_transfer(ctx)?;

    Ok(Response::new())
}
