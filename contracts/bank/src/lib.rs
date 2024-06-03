#[cfg(not(feature = "library"))]
use grug::grug_export;
use {
    anyhow::bail,
    grug::{
        grug_derive, Addr, BankQueryMsg, BankQueryResponse, Bound, Number, Coin, Coins,
        ImmutableCtx, Map, MutableCtx, Order, Response, StdResult, Storage, SudoCtx, TransferMsg,
        Uint128,
    },
    std::collections::{BTreeMap, HashMap},
};

// (address, denom) => balance
const BALANCES: Map<(&Addr, &str), Uint128> = Map::new("b");

// denom => supply
const SUPPLIES: Map<&str, Uint128> = Map::new("s");

// how many items to return in a paginated query by default
const DEFAULT_PAGE_LIMIT: u32 = 30;

#[grug_derive(serde)]
pub struct InstantiateMsg {
    pub initial_balances: BTreeMap<Addr, Coins>,
}

#[grug_derive(serde)]
pub enum ExecuteMsg {
    Mint {
        to: Addr,
        denom: String,
        amount: Uint128,
    },
    Burn {
        from: Addr,
        denom: String,
        amount: Uint128,
    },
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // need to make sure there are no duplicate address in initial balances.
    // we don't need to dedup denoms however. if there's duplicate denoms, the
    // deserialization setup should have already thrown an error.
    let mut supplies = HashMap::new();

    for (address, coins) in msg.initial_balances {
        for coin in coins {
            BALANCES.save(ctx.storage, (&address, &coin.denom), &coin.amount)?;
            accumulate_supply(&mut supplies, &coin.denom, coin.amount)?;
        }
    }

    for (denom, amount) in supplies {
        SUPPLIES.save(ctx.storage, &denom, &amount)?;
    }

    Ok(Response::new())
}

// just a helper function for use during instantiation
// not to be confused with `increase_supply` also found in this contract
fn accumulate_supply(
    supplies: &mut HashMap<String, Uint128>,
    denom: &str,
    by: Uint128,
) -> anyhow::Result<()> {
    let Some(supply) = supplies.get_mut(denom) else {
        supplies.insert(denom.into(), by);
        return Ok(());
    };

    *supply = supply.checked_add(by)?;

    Ok(())
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn bank_transfer(ctx: SudoCtx, msg: TransferMsg) -> StdResult<Response> {
    for coin in &msg.coins {
        decrease_balance(ctx.storage, &msg.from, coin.denom, *coin.amount)?;
        increase_balance(ctx.storage, &msg.to, coin.denom, *coin.amount)?;
    }

    Ok(Response::new()
        .add_attribute("method", "send")
        .add_attribute("from", msg.from)
        .add_attribute("to", msg.to)
        .add_attribute("coins", msg.coins.to_string()))
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    // we do not expect anyone to send any fund to this contract.
    // throw an error to revert the transfer.
    bail!("do not send funds to this contract");
}

#[cfg_attr(not(feature = "library"), grug_export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Mint { to, denom, amount } => mint(ctx, to, denom, amount),
        ExecuteMsg::Burn {
            from,
            denom,
            amount,
        } => burn(ctx, from, denom, amount),
    }
}

// NOTE: we haven't implement gatekeeping for minting/burning yet. for now
// anyone can mint any denom to any account, or burn any token from any account.
pub fn mint(ctx: MutableCtx, to: Addr, denom: String, amount: Uint128) -> anyhow::Result<Response> {
    increase_supply(ctx.storage, &denom, amount)?;
    increase_balance(ctx.storage, &to, &denom, amount)?;

    Ok(Response::new()
        .add_attribute("method", "mint")
        .add_attribute("to", to)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount))
}

// NOTE: we haven't implement gatekeeping for minting/burning yet. for now
// anyone can mint any denom to any account, or burn any token from any account.
pub fn burn(
    ctx: MutableCtx,
    from: Addr,
    denom: String,
    amount: Uint128,
) -> anyhow::Result<Response> {
    decrease_supply(ctx.storage, &denom, amount)?;
    decrease_balance(ctx.storage, &from, &denom, amount)?;

    Ok(Response::new()
        .add_attribute("method", "burn")
        .add_attribute("from", from)
        .add_attribute("denom", denom)
        .add_attribute("amount", amount))
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
        // if supply is reduced to zero, delete it, to save disk space
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
    BALANCES.update(storage, (address, denom), |balance| {
        let balance = balance.unwrap_or_default().checked_add(amount)?;
        Ok(Some(balance))
    })
}

/// Decrease an account's balance of a token by the given amount.
/// Return the balance value after the decrease.
fn decrease_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    denom: &str,
    amount: Uint128,
) -> StdResult<Option<Uint128>> {
    BALANCES.update(storage, (address, denom), |balance| {
        let balance = balance.unwrap_or_default().checked_sub(amount)?;
        // if balance is reduced to zero, delete it, to save disk space
        if balance.is_zero() {
            Ok(None)
        } else {
            Ok(Some(balance))
        }
    })
}

// Note to developers who wish to implement their own bank contracts:
// The query response MUST matches exactly the request. E.g. if the request is
// BankQuery::Balance, the response must be BankQueryResponse::Balance.
// It cannot be any other enum variant. Otherwise the chain may panic and halt.
#[cfg_attr(not(feature = "library"), grug_export)]
pub fn bank_query(ctx: ImmutableCtx, msg: BankQueryMsg) -> StdResult<BankQueryResponse> {
    match msg {
        BankQueryMsg::Balance { address, denom } => {
            query_balance(ctx, address, denom).map(BankQueryResponse::Balance)
        },
        BankQueryMsg::Balances {
            address,
            start_after,
            limit,
        } => query_balances(ctx, address, start_after, limit).map(BankQueryResponse::Balances),
        BankQueryMsg::Supply { denom } => query_supply(ctx, denom).map(BankQueryResponse::Supply),
        BankQueryMsg::Supplies { start_after, limit } => {
            query_supplies(ctx, start_after, limit).map(BankQueryResponse::Supplies)
        },
    }
}

pub fn query_balance(ctx: ImmutableCtx, address: Addr, denom: String) -> StdResult<Coin> {
    let maybe_amount = BALANCES.may_load(ctx.storage, (&address, &denom))?;
    Ok(Coin {
        denom,
        amount: maybe_amount.unwrap_or(Uint128::ZERO),
    })
}

pub fn query_balances(
    ctx: ImmutableCtx,
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
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit);
    Coins::from_iter_unchecked(&mut iter)
}

pub fn query_supply(ctx: ImmutableCtx, denom: String) -> StdResult<Coin> {
    let maybe_supply = SUPPLIES.may_load(ctx.storage, &denom)?;
    Ok(Coin {
        denom,
        amount: maybe_supply.unwrap_or(Uint128::ZERO),
    })
}

pub fn query_supplies(
    ctx: ImmutableCtx,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Coins> {
    let start = start_after
        .as_ref()
        .map(|denom| Bound::Exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;
    let mut iter = SUPPLIES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit);
    Coins::from_iter_unchecked(&mut iter)
}
