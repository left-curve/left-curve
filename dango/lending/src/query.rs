use {
    crate::{COLLATERAL_POWERS, DEBTS, MARKETS},
    dango_types::lending::{CollateralPower, Market, QueryMsg},
    grug::{
        Addr, Bound, Coins, Denom, ImmutableCtx, Inner, Json, JsonSerExt, NumberConst, Order,
        QuerierWrapper, StdResult, Storage, Udec128,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
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
        QueryMsg::UtilizationRate { account } => {
            let res = query_utilization_rate(ctx, account)?;
            res.to_json_value()
        },
    }
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

// Mock oracle price for now.
// TODO: Update once we finish oracle implementation.
// Perhaps we want to make a "oracle querier" that has a cache? So that we don't
// have to query the price for the same denom twice.
pub fn query_oracle_price(_querier: &QuerierWrapper, _denom: &Denom) -> StdResult<Udec128> {
    Ok(Udec128::ZERO)
}

/// Calculates the utilization rate of a margin account.
pub fn calculate_utilization_rate(
    querier: &QuerierWrapper,
    margin_account: Addr,
    debts: Coins,
    collateral_powers: BTreeMap<Denom, CollateralPower>,
) -> StdResult<Udec128> {
    // Calculate the total value of the debts.
    let mut total_debt_value = Udec128::ZERO;
    for coin in debts {
        let price = query_oracle_price(querier, &coin.denom)?;
        total_debt_value += coin.amount.checked_into_dec()? * price;
    }

    // Calculate the total value of the account's collateral adjusted for the collateral power.
    let mut total_adjusted_collateral_value = Udec128::ZERO;
    for (denom, power) in collateral_powers {
        let price = query_oracle_price(querier, &denom)?;
        let collateral_balance = querier.query_balance(margin_account, denom)?;
        let collateral_value = collateral_balance.checked_into_dec()? * price;
        total_adjusted_collateral_value += collateral_value * power.into_inner();
    }

    // Calculate the utilization rate.
    let utilization_rate = total_debt_value / total_adjusted_collateral_value;

    Ok(utilization_rate)
}

pub fn query_utilization_rate(ctx: ImmutableCtx, account: Addr) -> StdResult<Udec128> {
    // Query all debts for the account.
    let debts = query_debt(ctx.storage, account)?;

    // Query all collateral powers.
    let collateral_powers = query_collateral_powers(ctx.storage)?;

    calculate_utilization_rate(&ctx.querier, account, debts, collateral_powers)
}
