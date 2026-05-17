use {
    dango_types::account::InstantiateMsg,
    grug::{AuthCtx, MutableCtx, Response, Tx},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    dango_auth::create_account(ctx, msg.activate)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<Response> {
    dango_auth::authenticate_tx(ctx, tx, None)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(ctx: MutableCtx) -> anyhow::Result<Response> {
    dango_auth::receive_transfer(ctx)?;

    Ok(Response::new())
}
