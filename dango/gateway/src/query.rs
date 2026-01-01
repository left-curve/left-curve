use {
    crate::{RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES, WITHDRAWAL_FEES},
    dango_types::gateway::{QueryMsg, RateLimit, Remote},
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Json, JsonSerExt, Order, StdResult,
        Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Route { bridge, remote } => {
            let res = query_route(ctx, bridge, remote)?;
            res.to_json_value()
        },
        QueryMsg::ReverseRoute { denom, remote } => {
            let res = query_reverse_route(ctx, denom, remote)?;
            res.to_json_value()
        },
        QueryMsg::Routes { start_after, limit } => {
            let res = query_routes(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::RateLimits {} => {
            let res = query_rate_limits(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Reserve { bridge, remote } => {
            let res = query_reserve(ctx, bridge, remote)?;
            res.to_json_value()
        },
        QueryMsg::WithdrawalFee { denom, remote } => {
            let res = query_withdrawal_fee(ctx, denom, remote)?;
            res.to_json_value()
        },
    }
}

fn query_route(ctx: ImmutableCtx, bridge: Addr, remote: Remote) -> StdResult<Option<Denom>> {
    ROUTES.may_load(ctx.storage, (bridge, remote))
}

fn query_reverse_route(ctx: ImmutableCtx, denom: Denom, remote: Remote) -> StdResult<Option<Addr>> {
    REVERSE_ROUTES.may_load(ctx.storage, (&denom, remote))
}

fn query_routes(
    ctx: ImmutableCtx,
    start_after: Option<(Addr, Remote)>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<(Addr, Remote), Denom>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ROUTES
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit)
        .collect()
}

fn query_rate_limits(ctx: ImmutableCtx) -> StdResult<BTreeMap<Denom, RateLimit>> {
    RATE_LIMITS.load(ctx.storage)
}

fn query_reserve(ctx: ImmutableCtx, bridge: Addr, remote: Remote) -> StdResult<Uint128> {
    RESERVES.load(ctx.storage, (bridge, remote))
}

fn query_withdrawal_fee(ctx: ImmutableCtx, denom: Denom, remote: Remote) -> StdResult<Uint128> {
    WITHDRAWAL_FEES.load(ctx.storage, (&denom, remote))
}
