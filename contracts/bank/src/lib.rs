use cw_std::{
    cw_serde, entry_point, to_json, Addr, Binary, Bound, ExecuteCtx, InstantiateCtx, Map, Order,
    QueryCtx, Response, Uint128,
};

// (address, denom) => balance
// TODO: add an Addr type and replace address (&str) with &Addr
// TODO: add an Uint128 type and replace balance (u64) with Uint128
const BALANCES: Map<(&Addr, &str), Uint128> = Map::new("b");

// how many items to return in a paginated query by default
const DEFAULT_LIMIT: u32 = 30;

#[cw_serde]
pub struct InstantiateMsg {
    pub initial_balances: Vec<Balance>,
}

#[cw_serde]
pub struct Balance {
    pub address: Addr,
    pub denom:   String,
    pub amount:  Uint128,
}

#[cw_serde]
pub struct Coin {
    pub denom:   String,
    pub amount:  Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    Send {
        from:   Addr,
        to:     Addr,
        denom:  String,
        amount: Uint128,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Balance {
        address: Addr,
        denom:   String,
    },
    Balances {
        start_after: Option<(Addr, String)>, // (address, denom)
        limit:       Option<u32>,
    },
    BalancesByUser {
        address:     Addr,
        start_after: Option<String>, // denom
        limit:       Option<u32>
    },
}

#[entry_point]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for b in msg.initial_balances {
        // TODO: dedup
        BALANCES.save(ctx.store, (&b.address, &b.denom), &b.amount)?;
    }

    Ok(Response::new())
}

#[entry_point]
pub fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Send {
            from,
            to,
            denom,
            amount,
        } => send(ctx, from, to, denom, amount),
    }
}

#[entry_point]
pub fn query(ctx: QueryCtx, msg: QueryMsg) -> anyhow::Result<Binary> {
    match msg {
        QueryMsg::Balance {
            address,
            denom,
        } => to_json(&query_balance(ctx, address, denom)?),
        QueryMsg::Balances {
            start_after,
            limit,
        } => to_json(&query_balances(ctx, start_after, limit)?),
        QueryMsg::BalancesByUser {
            address,
            start_after,
            limit,
        } => to_json(&query_balances_by_user(ctx, address, start_after, limit)?),
    }
}

pub fn send(
    ctx:    ExecuteCtx,
    from:   Addr,
    to:     Addr,
    denom:  String,
    amount: Uint128,
) -> anyhow::Result<Response> {
    // decrease the sender's balance
    BALANCES.update(ctx.store, (&from, &denom), |maybe_balance| {
        let balance = maybe_balance.unwrap_or_else(Uint128::zero).checked_sub(amount)?;

        // if balance is reduced to zero, we delete it, to save disk space
        if balance > Uint128::zero() {
            Ok(Some(balance))
        } else {
            Ok(None)
        }
    })?;

    // increase the receiver's balance
    BALANCES.update(ctx.store, (&to, &denom), |maybe_balance| {
        maybe_balance.unwrap_or_else(Uint128::zero).checked_add(amount).map(Some)
    })?;

    Ok(Response::new())
}

pub fn query_balance(ctx: QueryCtx, address: Addr, denom: String) -> anyhow::Result<Balance> {
    let maybe_amount = BALANCES.may_load(ctx.store, (&address, &denom))?;
    Ok(Balance {
        address,
        denom,
        amount: maybe_amount.unwrap_or_else(Uint128::zero),
    })
}

pub fn query_balances(
    ctx:         QueryCtx,
    start_after: Option<(Addr, String)>,
    limit:       Option<u32>,
) -> anyhow::Result<Vec<Balance>> {
    let start = start_after.as_ref().map(|(addr, denom)| Bound::Exclusive((addr, denom.as_str())));
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;

    BALANCES
        .range(ctx.store, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let ((address, denom), amount) = item?;
            Ok(Balance {
                address,
                denom,
                amount,
            })
        })
        .collect()
}

pub fn query_balances_by_user(
    ctx:         QueryCtx,
    address:     Addr,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<Vec<Coin>> {
    let start = start_after.as_ref().map(|denom| Bound::Exclusive(denom.as_str()));
    let limit = limit.unwrap_or(DEFAULT_LIMIT) as usize;

    BALANCES
        .prefix(&address)
        .range(ctx.store, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin {
                denom,
                amount,
            })
        })
        .collect()
}
