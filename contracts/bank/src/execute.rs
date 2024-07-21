use {
    crate::{BALANCES_BY_ADDR, BALANCES_BY_DENOM, SUPPLIES},
    grug_types::{Addr, Coins, MutableCtx, Number, Response, StdResult, Storage, Uint128},
    std::collections::HashMap,
};

pub fn initialize<B>(storage: &mut dyn Storage, initial_balances: B) -> StdResult<Response>
where
    B: IntoIterator<Item = (Addr, Coins)>,
{
    // Need to make sure there are no duplicate address in initial balances.
    // We don't need to dedup denoms however. If there's duplicate denoms, the
    // deserialization setup should have already thrown an error.
    let mut supplies = HashMap::new();

    for (address, coins) in initial_balances {
        for coin in coins {
            BALANCES_BY_ADDR.save(storage, (&address, &coin.denom), &coin.amount)?;
            BALANCES_BY_DENOM.save(storage, (&coin.denom, &address), &coin.amount)?;
            accumulate_supply(&mut supplies, &coin.denom, coin.amount)?;
        }
    }

    for (denom, amount) in supplies {
        SUPPLIES.save(storage, &denom, &amount)?;
    }

    Ok(Response::new())
}

// Just a helper function for use during instantiation.
// Not to be confused with `increase_supply` also found in this contract
fn accumulate_supply(
    supplies: &mut HashMap<String, Uint128>,
    denom: &str,
    by: Uint128,
) -> StdResult<()> {
    let Some(supply) = supplies.get_mut(denom) else {
        supplies.insert(denom.into(), by);
        return Ok(());
    };

    *supply = supply.checked_add(by)?;

    Ok(())
}

/// Mint tokens of specified denom and amount to an account.
///
/// NOTE: This demo contract doesn't implement any gatekeeping for minting,
/// meaning _any_ account can mint _any_ token of _any_ amount.
///
/// Apparently, this is not intended for using in production.
pub fn mint(ctx: MutableCtx, to: Addr, denom: String, amount: Uint128) -> StdResult<Response> {
    increase_supply(ctx.storage, &denom, amount)?;
    increase_balance(ctx.storage, &to, &denom, amount)?;

    Ok(Response::new()
        .add_attribute("method", "mint")
        .add_attribute("to", to)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount))
}

/// Burn tokens of specified denom and amount from an account.
///
/// NOTE: This demo contract doesn't implement any gatekeeping for burning,
/// meaning _any_ account can mint _any_ token of _any_ amount.
///
/// Apparently, this is not intended for using in production.
pub fn burn(ctx: MutableCtx, from: Addr, denom: String, amount: Uint128) -> StdResult<Response> {
    decrease_supply(ctx.storage, &denom, amount)?;
    decrease_balance(ctx.storage, &from, &denom, amount)?;

    Ok(Response::new()
        .add_attribute("method", "burn")
        .add_attribute("from", from)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount))
}

/// Transfer tokens from one account to another.
pub fn transfer(
    storage: &mut dyn Storage,
    from: &Addr,
    to: &Addr,
    coins: &Coins,
) -> StdResult<Response> {
    for coin in coins {
        decrease_balance(storage, from, coin.denom, *coin.amount)?;
        increase_balance(storage, to, coin.denom, *coin.amount)?;
    }

    Ok(Response::new()
        .add_attribute("method", "send")
        .add_attribute("from", from)
        .add_attribute("to", to)
        .add_attribute("coins", coins.to_string()))
}

/// Increase the total supply of a token by the given amount.
/// Return the total supply value after the increase.
fn increase_supply(
    storage: &mut dyn Storage,
    denom: &str,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    SUPPLIES.update(storage, denom, |supply| {
        let supply = supply.unwrap_or_default().checked_add(amount)?;
        Ok(Some(supply))
    })
}

/// Decrease the total supply of a token by the given amount.
/// Return the total supply value after the decrease.
fn decrease_supply(
    storage: &mut dyn Storage,
    denom: &str,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    SUPPLIES.update(storage, denom, |supply| {
        let supply = supply.unwrap_or_default().checked_sub(amount)?;
        // If supply is reduced to zero, delete it, to save disk space.
        if supply.is_zero() {
            Ok(None)
        } else {
            Ok(Some(supply))
        }
    })
}

/// Increase an account's balance of a token by the given amount.
/// Return the balance value after the increase.
fn increase_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &str,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    let action = |balance: Option<Uint128>| {
        let balance = balance.unwrap_or_default().checked_add(amount)?;
        Ok(Some(balance))
    };
    BALANCES_BY_ADDR.update(storage, (address, denom), action)?;
    BALANCES_BY_DENOM.update(storage, (denom, address), action)
}

/// Decrease an account's balance of a token by the given amount.
/// Return the balance value after the decrease.
fn decrease_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &str,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    let action = |balance: Option<Uint128>| {
        let balance = balance.unwrap_or_default().checked_sub(amount)?;
        // If balance is reduced to zero, delete it, to save disk space.
        if balance.is_zero() {
            Ok(None)
        } else {
            Ok(Some(balance))
        }
    };
    BALANCES_BY_ADDR.update(storage, (address, denom), action)?;
    BALANCES_BY_DENOM.update(storage, (denom, address), action)
}
