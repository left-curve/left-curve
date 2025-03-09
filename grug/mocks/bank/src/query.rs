use {
    crate::{BALANCES_BY_ADDR, BALANCES_BY_DENOM, SUPPLIES},
    grug_math::{NumberConst, Uint128},
    grug_types::{
        Addr, Bound, Coin, Coins, DEFAULT_PAGE_LIMIT, Denom, Order, QueryBalanceRequest,
        QueryBalancesRequest, QuerySuppliesRequest, QuerySupplyRequest, StdResult, Storage,
    },
    std::collections::BTreeMap,
};

pub fn query_balance(storage: &dyn Storage, req: QueryBalanceRequest) -> StdResult<Coin> {
    let maybe_amount = BALANCES_BY_ADDR.may_load(storage, (req.address, &req.denom))?;

    Ok(Coin {
        denom: req.denom,
        amount: maybe_amount.unwrap_or(Uint128::ZERO),
    })
}

pub fn query_balances(storage: &dyn Storage, req: QueryBalancesRequest) -> StdResult<Coins> {
    let start = req.start_after.as_ref().map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    BALANCES_BY_ADDR
        .prefix(req.address)
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<BTreeMap<_, _>>>()?
        .try_into()
}

pub fn query_supply(storage: &dyn Storage, req: QuerySupplyRequest) -> StdResult<Coin> {
    let maybe_supply = SUPPLIES.may_load(storage, &req.denom)?;

    Ok(Coin {
        denom: req.denom,
        amount: maybe_supply.unwrap_or(Uint128::ZERO),
    })
}

pub fn query_supplies(storage: &dyn Storage, req: QuerySuppliesRequest) -> StdResult<Coins> {
    let start = req.start_after.as_ref().map(Bound::Exclusive);
    let limit = req.limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    SUPPLIES
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<BTreeMap<_, _>>>()?
        .try_into()
}

pub fn query_holders(
    storage: &dyn Storage,
    denom: Denom,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, Uint128>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    BALANCES_BY_DENOM
        .prefix(&denom)
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
