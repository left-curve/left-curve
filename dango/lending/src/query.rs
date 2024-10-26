use {
    crate::{LIABILITIES, WHITELISTED_DENOMS},
    dango_types::lending::QueryMsg,
    grug::{Addr, Bound, Coins, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::WhitelistedDenoms { limit, start_after } => {
            let res = query_whitelisted_denoms(ctx.storage, limit, start_after)?;
            res.to_json_value()
        },
        QueryMsg::DebtsOfAccount(addr) => {
            let res = query_debts(ctx.storage, addr)?;
            res.to_json_value()
        },
        QueryMsg::Liabilities { limit, start_after } => {
            let res = query_liabilities(ctx.storage, limit, start_after)?;
            res.to_json_value()
        },
    }
}

/// Queries the whitelisted denoms.
pub fn query_whitelisted_denoms(
    storage: &dyn Storage,
    limit: Option<u32>,
    start_after: Option<Denom>,
) -> StdResult<Vec<Denom>> {
    let iter = WHITELISTED_DENOMS.range(
        storage,
        start_after.map(Bound::Exclusive),
        None,
        Order::Descending,
    );

    let res = if let Some(limit) = limit {
        iter.take(limit as usize).collect::<StdResult<Vec<_>>>()?
    } else {
        iter.collect::<StdResult<Vec<_>>>()?
    };

    Ok(res)
}

/// Queries the debts of a margin account.
pub fn query_debts(storage: &dyn Storage, addr: Addr) -> StdResult<Coins> {
    let debts = LIABILITIES.load(storage, addr)?;
    Ok(debts)
}

/// Queries the liabilities of the lending pool.
pub fn query_liabilities(
    storage: &dyn Storage,
    limit: Option<u32>,
    start_after: Option<Addr>,
) -> StdResult<Vec<(Addr, Coins)>> {
    let iter = LIABILITIES.range(
        storage,
        start_after.map(Bound::Exclusive),
        None,
        Order::Ascending,
    );

    let res = if let Some(limit) = limit {
        iter.take(limit as usize).collect::<StdResult<Vec<_>>>()?
    } else {
        iter.collect::<StdResult<Vec<_>>>()?
    };

    Ok(res)
}
