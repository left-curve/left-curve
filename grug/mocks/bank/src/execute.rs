use {
    crate::{BALANCES_BY_ADDR, BALANCES_BY_DENOM, SUPPLIES},
    anyhow::ensure,
    grug_math::{IsZero, Number, Uint128},
    grug_types::{Addr, Coins, Denom, MutableCtx, QuerierExt, Response, StdResult, Storage},
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
            BALANCES_BY_ADDR.save(storage, (address, &coin.denom), &coin.amount)?;
            BALANCES_BY_DENOM.save(storage, (&coin.denom, address), &coin.amount)?;
            accumulate_supply(&mut supplies, coin.denom, coin.amount)?;
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
    supplies: &mut HashMap<Denom, Uint128>,
    denom: Denom,
    by: Uint128,
) -> StdResult<()> {
    let Some(supply) = supplies.get_mut(&denom) else {
        supplies.insert(denom, by);
        return Ok(());
    };

    supply.checked_add_assign(by)?;

    Ok(())
}

/// Mint tokens of specified denom and amount to an account.
///
/// NOTE: This demo contract doesn't implement any gatekeeping for minting,
/// meaning _any_ account can mint _any_ token of _any_ amount.
///
/// Apparently, this is not intended for using in production.
pub fn mint(ctx: MutableCtx, to: Addr, denom: Denom, amount: Uint128) -> anyhow::Result<Response> {
    increase_supply(ctx.storage, &denom, amount)?;
    increase_balance(ctx.storage, to, &denom, amount)?;

    Ok(Response::new())
}

/// Burn tokens of specified denom and amount from an account.
///
/// NOTE: This demo contract doesn't implement any gatekeeping for burning,
/// meaning _any_ account can mint _any_ token of _any_ amount.
///
/// Apparently, this is not intended for using in production.
pub fn burn(
    ctx: MutableCtx,
    from: Addr,
    denom: Denom,
    amount: Uint128,
) -> anyhow::Result<Response> {
    decrease_supply(ctx.storage, &denom, amount)?;
    decrease_balance(ctx.storage, from, &denom, amount)?;

    Ok(Response::new())
}

pub fn force_transfer(
    ctx: MutableCtx,
    from: Addr,
    to: Addr,
    denom: Denom,
    amount: Uint128,
) -> anyhow::Result<Response> {
    // Only the taxman can force transfer.
    ensure!(
        ctx.sender == ctx.querier.query_taxman()?,
        "you don't have the right, O you don't have the right"
    );

    decrease_balance(ctx.storage, from, &denom, amount)?;
    increase_balance(ctx.storage, to, &denom, amount)?;

    Ok(Response::new())
}

/// Transfer tokens from one account to another.
pub fn transfer(storage: &mut dyn Storage, from: Addr, to: Addr, coins: &Coins) -> StdResult<()> {
    for coin in coins {
        decrease_balance(storage, from, coin.denom, *coin.amount)?;
        increase_balance(storage, to, coin.denom, *coin.amount)?;
    }

    Ok(())
}

/// Increase the total supply of a token by the given amount.
/// Return the total supply value after the increase.
fn increase_supply(
    storage: &mut dyn Storage,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    SUPPLIES.may_modify(storage, denom, |supply| {
        let supply = supply.unwrap_or_default().checked_add(amount)?;
        // Only write to storage if the supply is non-zero.
        if supply.is_zero() {
            Ok(None)
        } else {
            Ok(Some(supply))
        }
    })
}

/// Decrease the total supply of a token by the given amount.
/// Return the total supply value after the decrease.
fn decrease_supply(
    storage: &mut dyn Storage,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    SUPPLIES.may_modify(storage, denom, |supply| {
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
    address: Addr,
    denom: &Denom,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    let action = |balance: Option<Uint128>| {
        let balance = balance.unwrap_or_default().checked_add(amount)?;
        // Only write to storage if the balance is non-zero.
        if balance.is_zero() {
            Ok(None)
        } else {
            Ok(Some(balance))
        }
    };

    BALANCES_BY_ADDR.may_modify(storage, (address, denom), action)?;
    BALANCES_BY_DENOM.may_modify(storage, (denom, address), action)
}

/// Decrease an account's balance of a token by the given amount.
/// Return the balance value after the decrease.
fn decrease_balance(
    storage: &mut dyn Storage,
    address: Addr,
    denom: &Denom,
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

    BALANCES_BY_ADDR.may_modify(storage, (address, denom), action)?;
    BALANCES_BY_DENOM.may_modify(storage, (denom, address), action)
}
