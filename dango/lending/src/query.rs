use {
    crate::{COLLATERAL_POWERS, DEBTS, MARKETS},
    anyhow::bail,
    dango_types::lending::{CollateralPower, HealthResponse, Market, QueryMsg},
    grug::{
        Addr, Bound, Coins, Denom, ImmutableCtx, Inner, IsZero, Json, JsonSerExt, NumberConst,
        Order, QuerierWrapper, StdResult, Storage, Udec128,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Market { denom } => {
            let res = query_market(ctx.storage, denom)?;
            res.to_json_value()
        },
        QueryMsg::Markets { start_after, limit } => {
            let res = query_markets(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Debt { account } => {
            let res = query_debt(ctx.storage, account)?;
            res.to_json_value()
        },
        QueryMsg::Debts { start_after, limit } => {
            let res = query_debts(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::CollateralPowers {} => {
            let res = query_collateral_powers(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Health { account } => {
            let res = query_account_health(ctx, account)?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

pub fn query_market(storage: &dyn Storage, denom: Denom) -> StdResult<Market> {
    MARKETS.load(storage, &denom)
}

pub fn query_markets(
    storage: &dyn Storage,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Market>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    MARKETS
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

pub fn query_debt(storage: &dyn Storage, account: Addr) -> StdResult<Coins> {
    DEBTS.load(storage, account)
}

pub fn query_debts(
    storage: &dyn Storage,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, Coins>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    DEBTS
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

pub fn query_collateral_powers(
    storage: &dyn Storage,
) -> StdResult<BTreeMap<Denom, CollateralPower>> {
    COLLATERAL_POWERS.load(storage)
}

/// Calculates the health of a margin account.
pub fn calculate_account_health(
    querier: &QuerierWrapper,
    margin_account: Addr,
    debts: Coins,
    collateral_powers: BTreeMap<Denom, CollateralPower>,
) -> anyhow::Result<HealthResponse> {
    // Calculate the total value of the debts.
    let mut total_debt_value = Udec128::ZERO;
    for coin in debts {
        let price = dango_oracle::raw_query_price(querier, &coin.denom)?;
        total_debt_value += price.value_of_unit_amount(coin.amount);
    }

    // Calculate the total value of the account's collateral adjusted for the collateral power.
    let mut total_adjusted_collateral_value = Udec128::ZERO;
    for (denom, power) in collateral_powers {
        let collateral_balance = querier.query_balance(margin_account, denom.clone())?;

        // As an optimization, don't query the price if the collateral balance is zero.
        if collateral_balance.is_zero() {
            continue;
        }

        let price = dango_oracle::raw_query_price(querier, &denom)?;
        let collateral_value = price.value_of_unit_amount(collateral_balance);
        total_adjusted_collateral_value += collateral_value * power.into_inner();
    }

    if total_adjusted_collateral_value.is_zero() {
        bail!("The account has no collateral");
    }

    // Calculate the utilization rate.
    let utilization_rate = total_debt_value / total_adjusted_collateral_value;

    Ok(HealthResponse {
        utilization_rate,
        total_debt_value,
        total_adjusted_collateral_value,
    })
}

pub fn query_account_health(ctx: ImmutableCtx, account: Addr) -> anyhow::Result<HealthResponse> {
    // Query all debts for the account.
    let debts = query_debt(ctx.storage, account)?;

    // Query all collateral powers.
    let collateral_powers = query_collateral_powers(ctx.storage)?;

    calculate_account_health(&ctx.querier, account, debts, collateral_powers)
}
