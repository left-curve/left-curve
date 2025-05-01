use {
    crate::{ALLOYED_TO_UNDERLYING, UNDERLYING_TO_ALLOYED},
    dango_types::alloy::QueryMsg,
    grug::{Bound, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult},
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Alloy { underlying_denom } => {
            let res = query_alloy(ctx, underlying_denom)?;
            res.to_json_value()
        },
        QueryMsg::Alloys { start_after, limit } => {
            let res = query_alloys(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Dealloy { alloyed_denom } => {
            let res = query_dealloy(ctx, alloyed_denom)?;
            res.to_json_value()
        },
        QueryMsg::Dealloys { start_after, limit } => {
            let res = query_dealloys(ctx, start_after, limit)?;
            res.to_json_value()
        },
    }
}

fn query_alloy(ctx: ImmutableCtx, underlying_denom: Denom) -> StdResult<Option<Denom>> {
    UNDERLYING_TO_ALLOYED.may_load(ctx.storage, &underlying_denom)
}

fn query_alloys(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Denom>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    UNDERLYING_TO_ALLOYED
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_dealloy(ctx: ImmutableCtx, alloyed_denom: Denom) -> StdResult<Option<Denom>> {
    ALLOYED_TO_UNDERLYING.may_load(ctx.storage, &alloyed_denom)
}

fn query_dealloys(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Denom>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ALLOYED_TO_UNDERLYING
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}
