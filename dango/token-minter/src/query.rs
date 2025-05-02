use {
    crate::{ALLOYS, OUTBOUND_QUOTAS},
    dango_types::token_minter::QueryMsg,
    grug::{
        Bound, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult, Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::RateLimits {} => todo!(),
        QueryMsg::Alloy { underlying_denom } => query_alloy(ctx, underlying_denom)?.to_json_value(),
        QueryMsg::Alloys { start_after, limit } => {
            query_alloys(ctx, start_after, limit)?.to_json_value()
        },
        QueryMsg::OutboundQuota { denom } => query_outbound_quota(ctx, denom)?.to_json_value(),
        QueryMsg::OutboundQuotas { start_after, limit } => {
            query_outbound_quotas(ctx, start_after, limit)?.to_json_value()
        },
    }
}

#[inline]
fn query_alloy(ctx: ImmutableCtx, underlying_denom: Denom) -> StdResult<Denom> {
    ALLOYS.load(ctx.storage, &underlying_denom)
}

#[inline]
fn query_alloys(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Denom>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    ALLOYS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

#[inline]
fn query_outbound_quota(ctx: ImmutableCtx, denom: Denom) -> StdResult<Uint128> {
    OUTBOUND_QUOTAS.load(ctx.storage, &denom)
}

#[inline]
fn query_outbound_quotas(
    ctx: ImmutableCtx,
    start_after: Option<Denom>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Denom, Uint128>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    OUTBOUND_QUOTAS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}
