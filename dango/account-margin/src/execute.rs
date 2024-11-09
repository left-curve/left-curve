use {
    anyhow::{anyhow, ensure},
    dango_auth::authenticate_tx,
    dango_lending::{calculate_account_health, COLLATERAL_POWERS, DEBTS},
    dango_types::{
        account::InstantiateMsg,
        config::{ACCOUNT_FACTORY_KEY, LENDING_KEY},
        lending::CollateralPower,
    },
    grug::{
        Addr, AuthCtx, AuthResponse, BorshDeExt, Coins, Denom, MutableCtx, NumberConst, Response,
        StdResult, Tx, Udec128,
    },
    std::collections::BTreeMap,
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
    authenticate_tx(ctx, tx, None, None)?;

    Ok(AuthResponse::new().request_backrun(true))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn backrun(ctx: AuthCtx, _tx: Tx) -> anyhow::Result<Response> {
    let lending: Addr = ctx.querier.query_app_config(LENDING_KEY)?;

    // Query all debts for the account.
    let debts = ctx
        .querier
        .query_wasm_raw(lending, DEBTS.path(ctx.contract))?
        .map(|coins| coins.deserialize_borsh::<Coins>())
        .transpose()?
        .unwrap_or_default();

    // Query all collateral powers.
    let collateral_powers = ctx
        .querier
        .query_wasm_raw(lending, COLLATERAL_POWERS.path().clone())?
        .ok_or_else(|| anyhow!("collateral powers not found"))?
        .deserialize_borsh::<BTreeMap<Denom, CollateralPower>>()?;

    // Calculate the utilization rate.
    let health = calculate_account_health(&ctx.querier, ctx.contract, debts, collateral_powers)?;

    // If the utilization rate is greater than 1, the account is undercollateralized.
    ensure!(
        health.utilization_rate <= Udec128::ONE,
        "the action would make the account undercollateralized. Utilization rate after action: {}. Total debt: {}. Total collateral: {}",
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
