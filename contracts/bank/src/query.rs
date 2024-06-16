use {
    crate::{BALANCES, SUPPLIES},
    grug::{Addr, Bound, Coin, Coins, NumberConst, Order, StdResult, Storage, Uint128},
};

pub const DEFAULT_PAGE_LIMIT: u32 = 30;

pub fn query_balance(storage: &dyn Storage, address: Addr, denom: String) -> StdResult<Coin> {
    let maybe_amount = BALANCES.may_load(storage, (&address, &denom))?;
    Ok(Coin {
        denom,
        amount: maybe_amount.unwrap_or(Uint128::ZERO),
    })
}

pub fn query_balances(
    storage: &dyn Storage,
    address: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::Exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;
    let mut iter = BALANCES
        .prefix(&address)
        .range(storage, start, None, Order::Ascending)
        .take(limit);
    Coins::from_iter_unchecked(&mut iter)
}

pub fn query_supply(storage: &dyn Storage, denom: String) -> StdResult<Coin> {
    let maybe_supply = SUPPLIES.may_load(storage, &denom)?;
    Ok(Coin {
        denom,
        amount: maybe_supply.unwrap_or(Uint128::ZERO),
    })
}

pub fn query_supplies(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::Exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;
    let mut iter = SUPPLIES
        .range(storage, start, None, Order::Ascending)
        .take(limit);
    Coins::from_iter_unchecked(&mut iter)
}
