use {
    crate::{
        EPOCH, GLOBAL_OUTBOUND, RATE_LIMITS, RESERVES, REVERSE_ROUTES, ROUTES, SUPPLIES,
        USER_MOVEMENTS, WITHDRAWAL_FEES,
    },
    dango_types::{
        account_factory::UserIndex,
        gateway::{
            GlobalOutbound, Movement, QueryMsg, QueryReservesResponseItem, QueryRoutesResponseItem,
            QueryWithdrawalFeesResponseItem, RateLimit, Remote,
        },
    },
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Inner, Json, JsonSerExt,
        MultiplyFraction, Number, NumberConst, Order, StdResult, Uint128,
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
        QueryMsg::Epoch {} => {
            let res = query_epoch(ctx)?;
            res.to_json_value()
        },
        QueryMsg::Supply { denom } => {
            let res = query_supply(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::GlobalOutbound { denom } => {
            let res = query_global_outbound(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::UserMovement { user_index, denom } => {
            let res = query_user_movement(ctx, user_index, denom)?;
            res.to_json_value()
        },
        QueryMsg::AvailableWithdraw { denom } => {
            let res = query_global_available_withdraw(ctx, denom)?;
            res.to_json_value()
        },
        QueryMsg::AvailableWithdraws {} => {
            let res = query_global_available_withdraws(ctx)?;
            res.to_json_value()
        },
        QueryMsg::UserAvailableWithdraw { user_index, denom } => {
            let res = query_user_available_withdraw(ctx, user_index, denom)?;
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

fn query_epoch(ctx: ImmutableCtx) -> StdResult<u64> {
    EPOCH.load(ctx.storage)
}

fn query_supply(ctx: ImmutableCtx, denom: Denom) -> StdResult<Uint128> {
    SUPPLIES.load(ctx.storage, &denom)
}

fn query_global_outbound(ctx: ImmutableCtx, denom: Denom) -> StdResult<GlobalOutbound> {
    GLOBAL_OUTBOUND.load(ctx.storage, &denom)
}

fn query_user_movement(
    ctx: ImmutableCtx,
    user_index: UserIndex,
    denom: Denom,
) -> StdResult<Movement> {
    Ok(USER_MOVEMENTS
        .may_load(ctx.storage, (user_index, &denom))?
        .unwrap_or_default())
}

/// Computes the remaining global withdrawal allowance for a denom:
/// `supply * rate_limit - total_24h`, floored at zero.
/// Loads the snapshotted supply from storage; `total_24h` is the cached
/// rolling outbound from `GlobalOutbound`.
pub(crate) fn compute_global_available_withdraw(
    storage: &dyn grug::Storage,
    denom: &Denom,
    rate_limit: &RateLimit,
    total_24h: Uint128,
) -> StdResult<Uint128> {
    let supply = SUPPLIES.load(storage, denom)?;
    let daily_allowance = supply.checked_mul_dec_floor(rate_limit.into_inner())?;

    Ok(daily_allowance.saturating_sub(total_24h))
}

fn query_global_available_withdraw(ctx: ImmutableCtx, denom: Denom) -> StdResult<Option<Uint128>> {
    let rate_limits = RATE_LIMITS.load(ctx.storage)?;

    match rate_limits.get(&denom) {
        Some(rate_limit) => {
            let rolling = GLOBAL_OUTBOUND
                .may_load(ctx.storage, &denom)?
                .map(|g| g.total_24h)
                .unwrap_or(Uint128::ZERO);
            compute_global_available_withdraw(ctx.storage, &denom, rate_limit, rolling).map(Some)
        },
        None => Ok(None),
    }
}

fn query_global_available_withdraws(ctx: ImmutableCtx) -> StdResult<BTreeMap<Denom, Uint128>> {
    let rate_limits = RATE_LIMITS.load(ctx.storage)?;

    rate_limits
        .iter()
        .map(|(denom, rate_limit)| {
            let rolling = GLOBAL_OUTBOUND
                .may_load(ctx.storage, denom)?
                .map(|g| g.total_24h)
                .unwrap_or(Uint128::ZERO);
            let available =
                compute_global_available_withdraw(ctx.storage, denom, rate_limit, rolling)?;
            Ok((denom.clone(), available))
        })
        .collect()
}

fn query_user_available_withdraw(
    ctx: ImmutableCtx,
    _user_index: UserIndex,
    denom: Denom,
) -> StdResult<Option<Uint128>> {
    // All withdrawals count against the global limit — there is no per-user
    // deposit credit. The user's available is the same as the global available.
    let rate_limits = RATE_LIMITS.load(ctx.storage)?;

    match rate_limits.get(&denom) {
        Some(rate_limit) => {
            let rolling = GLOBAL_OUTBOUND
                .may_load(ctx.storage, &denom)?
                .map(|g| g.total_24h)
                .unwrap_or(Uint128::ZERO);
            compute_global_available_withdraw(ctx.storage, &denom, rate_limit, rolling).map(Some)
        },
        None => Ok(None),
    }
}
