use {
    anyhow::bail,
    cw_std::{
        cw_serde, entry_point, Addr, BankQuery, BankQueryResponse, Binary, Bound, Coin, Coins,
        ExecuteCtx, InstantiateCtx, Map, Order, QueryCtx, Response, TransferCtx, TransferMsg,
        Uint128,
    },
    std::collections::{HashMap, HashSet},
};

// (address, denom) => balance
const BALANCES: Map<(&Addr, &str), Uint128> = Map::new("b");

// denom => supply
const SUPPLIES: Map<&str, Uint128> = Map::new("s");

// how many items to return in a paginated query by default
const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cw_serde]
pub struct InstantiateMsg {
    pub initial_balances: Vec<Balance>,
}

#[cw_serde]
pub struct Balance {
    pub address: Addr,
    pub coins:   Coins,
}

#[cw_serde]
pub enum ExecuteMsg {
    // TODO
}

#[cw_serde]
pub enum QueryMsg {
    // TODO
}

#[entry_point]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // need to make sure there are no duplicate address in initial balances.
    // we don't need to dedup denoms however. if there's duplicate denoms, the
    // deserialization setup should have already thrown an error.
    let mut seen_addrs = HashSet::new();
    let mut supplies = HashMap::new();

    for Balance { address, coins } in msg.initial_balances {
        if seen_addrs.contains(&address) {
            bail!("Duplicate address in initial balances");
        }

        for coin in coins {
            BALANCES.save(ctx.store, (&address, &coin.denom), &coin.amount)?;
            increment_supply(&mut supplies, &coin.denom, coin.amount)?;
        }

        seen_addrs.insert(address);
    }

    for (denom, amount) in supplies {
        SUPPLIES.save(ctx.store, &denom, &amount)?;
    }

    Ok(Response::new())
}

fn increment_supply(
    supplies: &mut HashMap<String, Uint128>,
    denom:    &str,
    by:       Uint128,
) -> anyhow::Result<()> {
    let Some(supply) = supplies.get_mut(denom) else {
        supplies.insert(denom.into(), by);
        return Ok(());
    };

    *supply = supply.checked_add(by)?;

    Ok(())
}

#[entry_point]
pub fn transfer(ctx: TransferCtx, msg: TransferMsg) -> anyhow::Result<Response> {
    for coin in &msg.coins {
        // decrease the sender's balance
        BALANCES.update(ctx.store, (&msg.from, &coin.denom), |maybe_balance| {
            let balance = maybe_balance.unwrap_or_else(Uint128::zero).checked_sub(*coin.amount)?;
            // if balance is reduced to zero, we delete it, to save disk space
            if balance > Uint128::zero() {
                Ok(Some(balance))
            } else {
                Ok(None)
            }
        })?;

        // increase the receiver's balance
        BALANCES.update(ctx.store, (&msg.to, &coin.denom), |maybe_balance| {
            maybe_balance.unwrap_or_else(Uint128::zero).checked_add(*coin.amount).map(Some)
        })?;
    }

    Ok(Response::new()
        .add_attribute("method", "send")
        .add_attribute("from", msg.from)
        .add_attribute("to", msg.to)
        .add_attribute("coins", msg.coins.to_string()))
}

#[entry_point]
pub fn bank_query(ctx: QueryCtx, msg: BankQuery) -> anyhow::Result<BankQueryResponse> {
    match msg {
        BankQuery::Balance {
            address,
            denom,
        } => query_balance(ctx, address, denom).map(BankQueryResponse::Balance),
        BankQuery::Balances {
            address,
            start_after,
            limit,
        } => query_balances(ctx, address, start_after, limit).map(BankQueryResponse::Balances),
        BankQuery::Supply {
            denom,
        } => query_supply(ctx, denom).map(BankQueryResponse::Supply),
        BankQuery::Supplies {
            start_after,
            limit,
        } => query_supplies(ctx, start_after, limit).map(BankQueryResponse::Supplies),
    }
}

pub fn query_balance(ctx: QueryCtx, address: Addr, denom: String) -> anyhow::Result<Coin> {
    let maybe_amount = BALANCES.may_load(ctx.store, (&address, &denom))?;
    Ok(Coin {
        denom,
        amount: maybe_amount.unwrap_or_else(Uint128::zero),
    })
}

pub fn query_balances(
    ctx:         QueryCtx,
    address:     Addr,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<Coins> {
    let start = start_after.as_ref().map(|denom| Bound::Exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    BALANCES
        .prefix(&address)
        .range(ctx.store, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin { denom, amount })
        })
        .collect()
}

pub fn query_supply(ctx: QueryCtx, denom: String) -> anyhow::Result<Coin> {
    let maybe_supply = SUPPLIES.may_load(ctx.store, &denom)?;
    Ok(Coin {
        denom,
        amount: maybe_supply.unwrap_or_else(Uint128::zero),
    })
}

pub fn query_supplies(
    ctx:         QueryCtx,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<Coins> {
    let start = start_after.as_ref().map(|denom| Bound::Exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    SUPPLIES
        .range(ctx.store, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin { denom, amount })
        })
        .collect()
}

#[entry_point]
pub fn execute(_ctx: ExecuteCtx, _msg: ExecuteMsg) -> anyhow::Result<Response> {
    todo!()
}

#[entry_point]
pub fn query(_ctx: QueryCtx, _msg: QueryMsg) -> anyhow::Result<Binary> {
    todo!()
}
