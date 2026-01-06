use {
    crate::{RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES, WITHDRAWAL_FEES},
    dango_types::gateway::{
        QueryMsg, QueryReservesResponseItem, QueryRoutesResponseItem,
        QueryWithdrawalFeesResponseItem, RateLimit, Remote,
    },
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
        QueryMsg::Reserves { start_after, limit } => {
            let res = query_reserves(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::WithdrawalFee { denom, remote } => {
            let res = query_withdrawal_fee(ctx, denom, remote)?;
            res.to_json_value()
        },
        QueryMsg::WithdrawalFees { start_after, limit } => {
            let res = query_withdrawal_fees(ctx, start_after, limit)?;
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
) -> StdResult<Vec<QueryRoutesResponseItem>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ROUTES
        .range(ctx.storage, start, None, Order::Ascending)
        .map(|res| {
            let ((bridge, remote), denom) = res?;
            Ok(QueryRoutesResponseItem {
                bridge,
                remote,
                denom,
            })
        })
        .take(limit)
        .collect()
}

fn query_rate_limits(ctx: ImmutableCtx) -> StdResult<BTreeMap<Denom, RateLimit>> {
    RATE_LIMITS.load(ctx.storage)
}

fn query_reserve(ctx: ImmutableCtx, bridge: Addr, remote: Remote) -> StdResult<Uint128> {
    RESERVES.load(ctx.storage, (bridge, remote))
}

fn query_reserves(
    ctx: ImmutableCtx,
    start_after: Option<(Addr, Remote)>,
    limit: Option<u32>,
) -> StdResult<Vec<QueryReservesResponseItem>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    RESERVES
        .range(ctx.storage, start, None, Order::Ascending)
        .map(|res| {
            let ((bridge, remote), reserve) = res?;
            Ok(QueryReservesResponseItem {
                bridge,
                remote,
                reserve,
            })
        })
        .take(limit)
        .collect()
}

fn query_withdrawal_fee(ctx: ImmutableCtx, denom: Denom, remote: Remote) -> StdResult<Uint128> {
    WITHDRAWAL_FEES.load(ctx.storage, (&denom, remote))
}

fn query_withdrawal_fees(
    ctx: ImmutableCtx,
    start_after: Option<(Denom, Remote)>,
    limit: Option<u32>,
) -> StdResult<Vec<QueryWithdrawalFeesResponseItem>> {
    let start = start_after
        .as_ref()
        .map(|(denom, remote)| Bound::Exclusive((denom, *remote)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    WITHDRAWAL_FEES
        .range(ctx.storage, start, None, Order::Ascending)
        .map(|res| {
            let ((denom, remote), fee) = res?;
            Ok(QueryWithdrawalFeesResponseItem { denom, remote, fee })
        })
        .take(limit)
        .collect()
}
