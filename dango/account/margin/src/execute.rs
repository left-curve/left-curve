use {
    crate::core,
    anyhow::{anyhow, bail, ensure},
    dango_auth::authenticate_tx,
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account::{
            InstantiateMsg,
            margin::{ExecuteMsg, HealthResponse, Liquidate},
        },
        dex, lending,
    },
    grug::{
        AuthCtx, AuthResponse, Coins, Denom, Fraction, Inner, IsZero, Message, MutableCtx, Number,
        NumberConst, Response, StdResult, Tx, Udec128,
    },
    std::cmp::{max, min},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, _msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Only the account factory can create new accounts.
    ensure!(
        ctx.sender == ctx.querier.query_account_factory()?,
        "you don't have the right, O you don't have the right"
    );

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Liquidate { collateral } => liquidate(ctx, collateral),
    }
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn authenticate(ctx: AuthCtx, tx: Tx) -> anyhow::Result<AuthResponse> {
    authenticate_tx(ctx, tx, None)?;

    Ok(AuthResponse::new().request_backrun(true))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn backrun(ctx: AuthCtx, _tx: Tx) -> anyhow::Result<Response> {
    let oracle = ctx.querier.query_oracle()?;
    let mut oracle_querier = OracleQuerier::new_remote(oracle, ctx.querier);

    let health = core::query_and_compute_health(
        ctx.querier,
        &mut oracle_querier,
        ctx.contract,
        ctx.block.timestamp,
        None,
        true,
    )?;

    // After executing all messages in the transactions, the account must have
    // a utilization rate no greater than one. Otherwise, we throw an error to
    // revert the transaction.
    if let Some(HealthResponse {
        utilization_rate,
        total_debt_value,
        total_adjusted_collateral_value,
        ..
    }) = health
    {
        ensure!(
            utilization_rate <= Udec128::ONE,
            "this action would make account undercollateralized! utilization rate: {utilization_rate}, total debt: {total_debt_value}, total adjusted collateral: {total_adjusted_collateral_value}"
        );
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> StdResult<Response> {
    // Do nothing, accept all transfers.
    Ok(Response::new())
}

pub fn liquidate(ctx: MutableCtx, collateral_denom: Denom) -> anyhow::Result<Response> {
    let app_cfg = ctx.querier.query_dango_config()?;
    let mut oracle_querier = OracleQuerier::new_remote(app_cfg.addresses.oracle, ctx.querier);

    // Query account health
    let Some(HealthResponse {
        total_debt_value,
        utilization_rate,
        total_adjusted_collateral_value,
        debts,
        collaterals,
        limit_order_collaterals,
        ..
    }) = core::query_and_compute_health(
        ctx.querier,
        &mut oracle_querier,
        ctx.contract,
        ctx.block.timestamp,
        Some(ctx.funds.clone()),
        true,
    )?
    else {
        bail!("can't liquidate because the account doesn't have any debt");
    };

    // Ensure account is undercollateralized
    ensure!(
        utilization_rate > Udec128::ONE,
        "account is not undercollateralized! utilization rate: {utilization_rate}"
    );

    let health_factor = utilization_rate.checked_inv()?;
    let target_health_factor = app_cfg.target_utilization_rate.checked_inv()?;
    let liquidation_collateral_power = *app_cfg
        .collateral_powers
        .get(&collateral_denom)
        .ok_or_else(|| {
            anyhow!("collateral power not found for chosen collateral: `{collateral_denom}`")
        })?
        .inner();

    // Calculate liquidation bonus.
    let bonus_cap = total_adjusted_collateral_value
        .checked_div(total_debt_value.checked_mul(liquidation_collateral_power)?)?
        .saturating_sub(Udec128::ONE);
    let liq_bonus = max(
        *app_cfg.min_liquidation_bonus,
        min(
            bonus_cap,
            min(*app_cfg.max_liquidation_bonus, Udec128::ONE - health_factor),
        ),
    );

    // Calculate value of maximum repayable debt (MRD) to reach the target
    // utilization rate.
    //
    // It shouldn't be possible for the numerator to be negative, as the accunt
    // should only be liquidatable if it is undercollateralized. If the
    // denominator is negative (should only happen with an excessive minimum
    // liquidation bonus), then the MRD is set to the account's total debt value.
    //
    // See derivation of the equation in [liquidation-math.md](book/notes/liquidation-math.md).
    let mrd_to_target_health = if target_health_factor
        <= (Udec128::ONE + liq_bonus) * liquidation_collateral_power
    {
        total_debt_value
    } else {
        let numerator = total_debt_value
            .checked_mul(target_health_factor)?
            .checked_sub(total_adjusted_collateral_value)?;
        let denominator = target_health_factor
            .checked_sub((Udec128::ONE + liq_bonus).checked_mul(liquidation_collateral_power)?)?;
        numerator.checked_div(denominator)?
    };

    // Calculate the maximum debt that can be repaid based on the balance of the
    // chosen collateral.
    let collateral_price = oracle_querier.query_price(&collateral_denom, None)?;
    let liquidation_collateral_value = collateral_price.value_of_unit_amount(
        collaterals
            .amount_of(&collateral_denom)
            .checked_add(limit_order_collaterals.amount_of(&collateral_denom))?,
    )?;
    let mrd_from_chosen_collateral =
        liquidation_collateral_value.checked_div(Udec128::ONE + liq_bonus)?;

    // Calculate the debt value to repay.
    let debt_repay_value = min(
        total_debt_value,
        min(mrd_to_target_health, mrd_from_chosen_collateral),
    );

    ensure!(
        debt_repay_value.is_non_zero(),
        "debt repay value is zero! probably the account either has no debt, or no collateral."
    );

    // Repay the account's debts with the sent funds, up to the maximum value
    // of the repayable debt.
    let mut refunds = Coins::new();
    let mut repaid_debt_value = Udec128::ZERO;
    let mut repay_coins = Coins::new();

    for coin in ctx.funds {
        let debt_amount = debts.amount_of(&coin.denom);
        let price = oracle_querier.query_price(&coin.denom, None)?;
        let debt_value: Udec128 = price.value_of_unit_amount(debt_amount)?;

        let max_repay_for_denom = if repaid_debt_value.checked_add(debt_value)? > debt_repay_value {
            price.unit_amount_from_value(debt_repay_value.checked_sub(repaid_debt_value)?)?
        } else {
            debt_amount
        };

        let repay_amount = if coin.amount > max_repay_for_denom {
            refunds.insert((coin.denom.clone(), coin.amount - max_repay_for_denom))?;
            max_repay_for_denom
        } else {
            coin.amount
        };

        repay_coins.insert((coin.denom.clone(), repay_amount))?;
        repaid_debt_value.checked_add_assign(price.value_of_unit_amount::<18>(repay_amount)?)?;
    }

    // Ensure repaid debt value is not zero
    ensure!(repaid_debt_value.is_non_zero(), "no debt was repaid");

    // Calculate the amount of collateral to send to the liquidator. We round
    // up so that no dust is left in the account.
    let claimed_collateral_amount = collateral_price
        .unit_amount_from_value_ceil(repaid_debt_value.checked_mul(Udec128::ONE + liq_bonus)?)?;

    // Ensure liquidator receives a non-zero amount of collateral
    ensure!(
        claimed_collateral_amount.is_non_zero(),
        "liquidation would result in zero collateral claimed"
    );

    // Send the claimed collateral and any debt refunds to the liquidator.
    let mut send_coins = refunds.clone();
    send_coins.insert((collateral_denom.clone(), claimed_collateral_amount))?;
    let send_msg = Message::transfer(ctx.sender, send_coins)?;

    // Create message to repay debt
    let repay_msg = Message::execute(
        app_cfg.addresses.lending,
        &lending::ExecuteMsg::Repay {},
        repay_coins.clone(),
    )?;

    // Create message to cancel all the user's limit orders
    let cancel_msg = Message::execute(
        app_cfg.addresses.dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates_market: vec![],
            creates_limit: vec![],
            cancels: Some(dex::CancelOrderRequest::All),
        },
        Coins::new(),
    )?;

    Ok(Response::new()
        .add_message(cancel_msg)
        .add_message(repay_msg)
        .may_add_message(send_msg)
        .add_event(Liquidate {
            collateral_denom,
            repay_coins,
            refunds,
            repaid_debt_value,
            claimed_collateral_amount,
            liquidation_bonus: liq_bonus,
            target_health_factor,
        })?)
}
