use {
    anyhow::bail,
    cw_sdk::{cw_serde, entry_point, to_json, Binary, ExecuteCtx, Map, Response, QueryCtx},
};

// (address, denom) => balance
// TODO: add an Addr type and replace address (&str) with &Addr
// TODO: add an Uint128 type and replace balance (u64) with Uint128
const BALANCES: Map<(&str, &str), u64> = Map::new("b");

#[cw_serde]
pub enum ExecuteMsg {
    Send {
        from:   String,
        to:     String,
        denom:  String,
        amount: u64,
    },
}

#[cw_serde]
pub enum QueryMsg {
    Balance {
        address: String,
        denom:   String,
    },
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
    }
}

pub fn send(
    ctx:    ExecuteCtx,
    from:   String,
    to:     String,
    denom:  String,
    amount: u64,
) -> anyhow::Result<Response> {
    // decrease the sender's balance
    // if balance is reduced to zero, we delete it, to save disk space
    BALANCES.update(ctx.store, (&from, &denom), |maybe_balance| {
        let balance = maybe_balance.unwrap_or(0);
        let Some(balance) = balance.checked_sub(amount) else {
            bail!("Insufficient {denom} balance: {balance} < {amount}");
        };

        if balance > 0 {
            Ok(Some(balance))
        } else {
            Ok(None)
        }
    })?;

    // increase the receiver's balance
    BALANCES.update(ctx.store, (&to, &denom), |maybe_balance| {
        let balance = maybe_balance.unwrap_or(0);
        let Some(balance) = balance.checked_add(amount) else {
            bail!("Excessive {denom} balance: {balance} + {amount} > u64::MAX");
        };

        Ok(Some(balance))
    })?;

    Ok(Response::new())
}

fn query_balance(ctx: QueryCtx, addr: String, denom: String) -> anyhow::Result<u64> {
    BALANCES.may_load(ctx.store, (&addr, &denom)).map(|opt| opt.unwrap_or(0))
}
