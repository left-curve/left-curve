use {
    cw_sdk::{
        cw_serde, entry_point, to_json, Binary, ExecuteCtx, InstantiateCtx, Map, Order, QueryCtx,
        Response, Uint128,
    },
    std::ops::Bound,
};

// (address, denom) => balance
// TODO: add an Addr type and replace address (&str) with &Addr
// TODO: add an Uint128 type and replace balance (u64) with Uint128
const BALANCES: Map<(&str, &str), Uint128> = Map::new("b");

// how many items to return in a paginated query by default
const DEFAULT_LIMIT: u32 = 30;

#[cw_serde]
pub struct InstantiateMsg {
    pub initial_balances: Vec<Balance>,
}

#[cw_serde]
pub struct Balance {
    pub address: String,
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
        from:   String,
        to:     String,
        denom:  String,
        amount: Uint128,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Balance {
        address: String,
        denom:   String,
    },
    Balances {
        start_after: Option<(String, String)>, // (address, denom)
        limit:       Option<u32>,
    },
    BalancesByUser {
        address:     String,
        start_after: Option<String>, // denom
        limit:       Option<u32>
    },
}

#[entry_point]
pub fn instantiate(ctx: InstantiateCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for b in msg.initial_balances {
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
    from:   String,
    to:     String,
    denom:  String,
    amount: Uint128,
) -> anyhow::Result<Response> {
    // decrease the sender's balance
    // if balance is reduced to zero, we delete it, to save disk space
    BALANCES.update(ctx.store, (&from, &denom), |maybe_balance| {
        let balance = maybe_balance.unwrap_or_else(Uint128::zero).checked_sub(amount)?;

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

pub fn query_balance(ctx: QueryCtx, address: String, denom: String) -> anyhow::Result<Balance> {
    let maybe_amount = BALANCES.may_load(ctx.store, (&address, &denom))?;
    Ok(Balance {
        address,
        denom,
        amount: maybe_amount.unwrap_or_else(Uint128::zero),
    })
}

pub fn query_balances(
    ctx:         QueryCtx,
    start_after: Option<(String, String)>,
    limit:       Option<u32>,
) -> anyhow::Result<Vec<Balance>> {
    let min = match &start_after {
        Some((addr, denom)) => Bound::Excluded((addr.as_str(), denom.as_str())),
        None => Bound::Unbounded,
    };

    BALANCES
        .range(ctx.store, min, Bound::Unbounded, Order::Ascending)
        .take(limit.unwrap_or(DEFAULT_LIMIT) as usize)
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
    address:     String,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<Vec<Coin>> {
    let min = match &start_after {
        Some(denom) => Bound::Excluded(denom.as_str()),
        None => Bound::Unbounded,
    };

    BALANCES
        .prefix(&address)
        .range(ctx.store, min, Bound::Unbounded, Order::Ascending)
        .take(limit.unwrap_or(DEFAULT_LIMIT) as usize)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(Coin {
                denom,
                amount,
            })
        })
        .collect()
}
