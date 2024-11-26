use {
    crate::MarginQuerier,
    anyhow::{anyhow, ensure},
    dango_auth::authenticate_tx,
    dango_lending::DEBTS,
    dango_oracle::OracleQuerier,
    dango_types::{
        account::{
            margin::{ExecuteMsg, HealthResponse},
            InstantiateMsg,
        },
        config::AppConfig,
        DangoQuerier,
    },
    grug::{
        AuthCtx, AuthResponse, Coin, Coins, Denom, Fraction, Message, MutableCtx, NumberConst,
        Response, StdResult, Tx, Udec128,
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
    let health = ctx.querier.query_health(ctx.contract)?;

    // After executing all messages in the transactions, the account must have
    // a utilization rate no greater than one. Otherwise, we throw an error to
    // revert the transaction.
    ensure!(
        health.utilization_rate <= Udec128::ONE,
        "this action would make account undercollateralized! utilization rate: {}, total debt: {}, total adjusted collateral: {}",
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

pub fn liquidate(ctx: MutableCtx, liquidation_denom: Denom) -> anyhow::Result<Response> {
    // Query account health
    // ctx.querier.query_health(account)?;

    let app_cfg: AppConfig = ctx.querier.query_app_config()?;

    let HealthResponse {
        total_debt_value,
        utilization_rate,
        total_collateral_value,
        total_adjusted_collateral_value,
        debts,
    } = ctx.querier.query_health(ctx.contract)?;

    // Ensure account is undercollateralized
    ensure!(
        utilization_rate > Udec128::ONE,
        "account is not undercollateralized"
    );

    // Calculate liquidation bonus.
    // Liquidation bonus is defined as one minus health factor, bounded by the minimum
    // and maximum liquidation bonus variables.
    let health_factor = utilization_rate.checked_inv()?;
    let liq_bonus = min(
        max(*app_cfg.min_liquidation_bonus, Udec128::ONE - health_factor),
        *app_cfg.max_liquidation_bonus,
    );

    // Calculate value of maximum repayable debt (MRD) to reach the target utilization rate.
    // MRD is defined as:
    //
    //            (c - (1 + b) * d) * v
    // MRD = d -  ---------------------
    //               t - (1 + b) * v
    //
    // where:
    //       t = target health factor (the maximum health factor to liquidate to)
    //       v = average collateral power
    //       d = total debt value
    //       c = total collateral value
    //       b = liquidation bonus
    //
    // See derivation of above equation in [TODO: INSERT LINK]
    //
    let average_collateral_power = total_adjusted_collateral_value / total_collateral_value; // TODO: Ensure non-zero and not divide by zero
    let target_health_factor = app_cfg.target_utilization_rate.checked_inv()?;

    // If either numerator or denominator is negative, then no amount of debt repayment will make
    // the account reach the target health factor (bad debt accrues). In this case, we set the MRD
    // to the account's total debt value.
    let mrd_to_target_health = if total_collateral_value
        < (Udec128::ONE + liq_bonus) * total_debt_value
        || target_health_factor <= (Udec128::ONE + liq_bonus) * average_collateral_power
    {
        total_debt_value
    } else {
        let numerator = (total_collateral_value - (Udec128::ONE + liq_bonus) * total_debt_value)
            * average_collateral_power;
        let denominator =
            target_health_factor - (Udec128::ONE + liq_bonus) * average_collateral_power;

        numerator / denominator
    };

    // Calculate the maximum debt that can be repaid based on the balance of the
    // chosen collateral.
    let collateral_price = ctx
        .querier
        .query_price(app_cfg.addresses.oracle, &liquidation_denom)?;
    let liquidation_collateral_value = collateral_price.value_of_unit_amount(
        ctx.querier
            .query_balance(ctx.contract, liquidation_denom.clone())?,
    )?;
    let mrd_from_chosen_collateral = liquidation_collateral_value / (Udec128::ONE + liq_bonus);

    // Calculate the debt value to repay.
    let debt_repay_value = [
        mrd_to_target_health,
        mrd_from_chosen_collateral,
        total_debt_value,
    ]
    .into_iter()
    .min()
    .ok_or_else(|| anyhow!("unable to calculate debt repay value"))?;

    // Repay the account's debts with the sent funds, up to the maximum value
    // of the repayable debt.
    let mut refunds = Coins::new();
    let mut repaid_debt_value = Udec128::ZERO;
    let mut repay_coins = Coins::new();
    for coin in ctx.funds {
        let debt_amount = debts.amount_of(&coin.denom);
        let price = ctx
            .querier
            .query_price(app_cfg.addresses.oracle, &coin.denom)?;
        let debt_value = price.value_of_unit_amount(debt_amount)?;

        let max_repay_for_denom = if repaid_debt_value + debt_value > debt_repay_value {
            price.unit_amount_from_value(debt_repay_value - repaid_debt_value)?
        } else {
            debt_amount
        };

        let repay_amount = if coin.amount > max_repay_for_denom {
            refunds.insert(Coin::new(
                coin.denom.clone(),
                coin.amount - max_repay_for_denom,
            )?)?;
            max_repay_for_denom
        } else {
            coin.amount
        };

        repay_coins.insert(Coin::new(coin.denom.clone(), repay_amount)?)?;
        repaid_debt_value += price.value_of_unit_amount(repay_amount)?;
    }
    DEBTS.may_update(ctx.storage, ctx.sender, |maybe_debts| {
        let mut debts = maybe_debts.unwrap_or_default();

        debts.saturating_deduct_many(repay_coins)?;

        Ok::<Coins, anyhow::Error>(debts)
    })?;

    // Calculate the amount of collateral to send to the liquidator.
    let collateral_price = ctx
        .querier
        .query_price(app_cfg.addresses.oracle, &liquidation_denom)?;
    let claimed_collateral =
        collateral_price.unit_amount_from_value(repaid_debt_value * (Udec128::ONE + liq_bonus))?;

    // Send the claimed collateral and any debt refunds to the liquidator.
    refunds.insert(Coin::new(liquidation_denom.clone(), claimed_collateral)?)?;

    Ok(Response::new().add_message(Message::transfer(ctx.sender, refunds)?))
}
