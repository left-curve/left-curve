use {
    anyhow::bail,
    cw_sdk::{cw_serde, entry_point, ExecuteCtx, Map, Response},
};

const BALANCES: Map<String, u64> = Map::new("b");

#[cw_serde]
pub enum ExecuteMsg {
    Send {
        from:   String,
        to:     String,
        amount: u64,
    },
}

#[entry_point]
pub fn execute(ctx: ExecuteCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::Send {
            from,
            to,
            amount,
        } => send(ctx, from, to, amount),
    }
}

pub fn send(
    ctx:    ExecuteCtx,
    from:   String,
    to:     String,
    amount: u64,
) -> anyhow::Result<Response> {
    // decrease the sender's balance
    // if balance is reduced to zero, we delete it, to save disk space
    BALANCES.update(ctx.store, &from, |maybe_balance| {
        let balance = maybe_balance.unwrap_or(0);
        let Some(balance) = balance.checked_sub(amount) else {
            bail!("Insufficient balance: {balance} < {amount}");
        };

        if balance > 0 {
            Ok(Some(balance))
        } else {
            Ok(None)
        }
    })?;

    // increase the receiver's balance
    BALANCES.update(ctx.store, &to, |maybe_balance| {
        let balance = maybe_balance.unwrap_or(0);
        let Some(balance) = balance.checked_add(amount) else {
            bail!("Excessive balance: {balance} + {amount} > u64::MAX")
        };

        Ok(Some(balance))
    })?;

    Ok(Response::new())
}
