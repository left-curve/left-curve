use {
    anyhow::{bail, Context},
    cw_sdk::{cw_serde, entry_point, ExecuteCtx, Response, Storage},
};

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
    Balance::decrease(ctx.store, &from, amount)?;
    Balance::increase(ctx.store, &to, amount)?;

    Ok(Response::new())
}

pub struct Balance;

impl Balance {
    pub fn increase(store: &mut dyn Storage, addr: &str, amount: u64) -> anyhow::Result<()> {
        let balance_before = Self::get(store, addr)?;

        let balance_after = balance_before
            .checked_add(amount)
            .with_context(|| format!("Excessive balance: {balance_before} + {amount} > u64::MAX"))?;

        Self::set(store, addr, balance_after)
    }

    pub fn decrease(store: &mut dyn Storage, addr: &str, amount: u64) -> anyhow::Result<()> {
        let balance_before = Self::get(store, addr)?;

        let balance_after = balance_before
            .checked_sub(amount)
            .with_context(|| format!("Insufficient balance: {balance_before} < {amount}"))?;

        if balance_after > 0 {
            Self::set(store, addr, balance_after)
        } else {
            Self::remove(store, addr)
        }
    }

    fn get(store: &dyn Storage, addr: &str) -> anyhow::Result<u64> {
        let Some(balance_bytes) = store.read(addr.as_bytes()) else {
            return Ok(0);
        };
        let Ok(balance_bytes) = <[u8; 8]>::try_from(balance_bytes) else {
            bail!("Failed to parse balance: expect 8 bytes");
        };
        Ok(u64::from_be_bytes(balance_bytes))
    }

    fn set(store: &mut dyn Storage, addr: &str, amount: u64) -> anyhow::Result<()> {
        store.write(addr.as_bytes(), &amount.to_be_bytes());
        Ok(())
    }

    fn remove(store: &mut dyn Storage, addr: &str) -> anyhow::Result<()> {
        store.remove(addr.as_bytes());
        Ok(())
    }
}
