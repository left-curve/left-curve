use {
    crate::query_health,
    anyhow::ensure,
    dango_auth::authenticate_tx,
    dango_types::{account::InstantiateMsg, config::AppConfig},
    grug::{AuthCtx, AuthResponse, MutableCtx, NumberConst, Response, StdResult, Tx, Udec128},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    let app_cfg: AppConfig = ctx.querier.query_app_config()?;

    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == app_cfg.addresses.account_factory,
        "you don't have the right, O you don't have the right"
    );

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    authenticate_tx(ctx, tx, None, None)?;

    Ok(AuthResponse::new().request_backrun(true))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn backrun(ctx: AuthCtx, _tx: Tx) -> anyhow::Result<Response> {
    let health = query_health(ctx.into_immutable())?;

    // If the utilization rate is greater than 1, the account is undercollateralized.
    ensure!(
        health.utilization_rate <= Udec128::ONE,
        "the action would make the account undercollateralized! utilization rate after action: {}, total debt: {}, total collateral: {}",
        health.utilization_rate,
        health.total_debt_value,
        health.total_adjusted_collateral_value
    );

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    // Do nothing, accept all transfers.
    Ok(Response::new())
}
