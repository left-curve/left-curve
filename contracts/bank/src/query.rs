use {
    crate::{BALANCES_BY_ADDR, BALANCES_BY_DENOM, SUPPLIES},
    grug_math::{NumberConst, Uint256},
    grug_storage::Bound,
    grug_types::{Addr, Coin, Coins, Denom, Order, StdResult, Storage},
    std::collections::BTreeMap,
};

pub const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_balance(storage: &dyn Storage, address: Addr, denom: Denom) -> StdResult<Coin> {
    let maybe_amount = BALANCES_BY_ADDR.may_load(storage, (address, &denom))?;
    Ok(Coin {
        denom,
        amount: maybe_amount.unwrap_or(Uint256::ZERO),
    })
}

pub fn query_balances(
    storage: &dyn Storage,
    address: Addr,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    BALANCES_BY_ADDR
        .prefix(address)
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<BTreeMap<_, _>>>()?
        .try_into()
}

pub fn query_supply(storage: &dyn Storage, denom: Denom) -> StdResult<Coin> {
    let maybe_supply = SUPPLIES.may_load(storage, &denom)?;
    Ok(Coin {
        denom,
        amount: maybe_supply.unwrap_or(Uint256::ZERO),
    })
}

pub fn query_supplies(
    storage: &dyn Storage,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

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
) -> StdResult<BTreeMap<Addr, Uint256>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    BALANCES_BY_DENOM
        .prefix(&denom)
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
