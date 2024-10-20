use {
    crate::{CONFIG, DENOM_ADMINS},
    dango_types::token_factory::{Config, QueryMsg},
    grug::{Addr, Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Storage},
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Config {} => {
            let res = query_config(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::Admin { denom } => {
            let res = query_admin(ctx.storage, denom)?;
            res.to_json_value()
        },
        QueryMsg::Admins { start_after, limit } => {
            let res = query_admins(ctx.storage, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

fn query_admin(storage: &dyn Storage, denom: Denom) -> StdResult<Addr> {
    DENOM_ADMINS.load(storage, &denom)
}

fn query_admins(
    storage: &dyn Storage,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<Vec<(Denom, Addr)>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    DENOM_ADMINS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}
