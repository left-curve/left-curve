use {
    crate::{LIABILITIES, MARKETS},
    dango_types::lending::{Market, QueryMsg},
    grug::{Addr, Bound, Coins, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Market { denom } => {
            let res = query_market(ctx.storage, denom)?;
            res.to_json_value()
        },
        QueryMsg::Markets { start_after, limit } => {
            let res = query_markets(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::DebtsOfAccount(addr) => {
            let res = query_debts(ctx.storage, addr)?;
            res.to_json_value()
        },
        QueryMsg::Liabilities { start_after, limit } => {
            let res = query_liabilities(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
    }
}

pub fn query_market(storage: &dyn Storage, denom: Denom) -> StdResult<Market> {
    MARKETS.load(storage, &denom)
}

pub fn query_markets(
    storage: &dyn Storage,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Market>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    MARKETS
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

/// Queries the debts of a margin account.
pub fn query_debts(storage: &dyn Storage, addr: Addr) -> StdResult<Coins> {
    LIABILITIES.load(storage, addr)
}

/// Queries the liabilities of the lending pool.
pub fn query_liabilities(
    storage: &dyn Storage,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<Vec<(Addr, Coins)>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    LIABILITIES
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
