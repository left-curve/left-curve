use {
    crate::{ASKS, BIDS, OrderKey, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES},
    dango_types::perps::{
        Order, OrderId, PairId, PairParam, PairState, QueryMsg, QueryOrderResponse,
        QueryOrdersByUserResponse, UserState,
    },
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, ImmutableCtx, Json, JsonSerExt, Order as IterationOrder,
        StdResult,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Param {} => {
            let res = PARAM.load(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::PairParam { pair_id } => {
            let res = PAIR_PARAMS.may_load(ctx.storage, &pair_id)?;
            res.to_json_value()
        },
        QueryMsg::PairParams { start_after, limit } => {
            let res = query_pair_params(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::State {} => {
            let res = STATE.load(ctx.storage)?;
            res.to_json_value()
        },
        QueryMsg::PairState { pair_id } => {
            let res = PAIR_STATES.may_load(ctx.storage, &pair_id)?;
            res.to_json_value()
        },
        QueryMsg::PairStates { start_after, limit } => {
            let res = query_pair_states(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::UserState { user } => {
            let res = USER_STATES.may_load(ctx.storage, user)?;
            res.to_json_value()
        },
        QueryMsg::UserStates { start_after, limit } => {
            let res = query_user_states(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Order { order_id } => {
            let res = query_order(ctx, order_id)?;
            res.to_json_value()
        },
        QueryMsg::OrdersByUser { user } => {
            let res = query_orders_by_user(ctx, user)?;
            res.to_json_value()
        },
    }
}

fn query_pair_params(
    ctx: ImmutableCtx,
    start_after: Option<PairId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<PairId, PairParam>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    PAIR_PARAMS
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .collect()
}

fn query_pair_states(
    ctx: ImmutableCtx,
    start_after: Option<PairId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<PairId, PairState>> {
    let start = start_after.as_ref().map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    PAIR_STATES
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .collect()
}

fn query_user_states(
    ctx: ImmutableCtx,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<Addr, UserState>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    USER_STATES
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .collect()
}

/// We don't know if the order is a buy or a sell.
/// First we look for it in the `BIDS` map. If non-exists, we look for it in the
/// `ASKS` map. If still non-exists, return `None.`
fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<Option<QueryOrderResponse>> {
    if let Some((order_key, order)) = BIDS.idx.order_id.may_load(ctx.storage, order_id)? {
        return Ok(Some(into_query_order_response((order_key, order))));
    }

    if let Some((order_key, order)) = ASKS.idx.order_id.may_load(ctx.storage, order_id)? {
        return Ok(Some(into_query_order_response((order_key, order))));
    }

    Ok(None)
}

fn query_orders_by_user(ctx: ImmutableCtx, user: Addr) -> StdResult<QueryOrdersByUserResponse> {
    let bids = BIDS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(try_into_query_order_response)
        .collect::<StdResult<Vec<_>>>()?;

    let asks = ASKS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(try_into_query_order_response)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(QueryOrdersByUserResponse { bids, asks })
}

fn try_into_query_order_response(
    res: StdResult<(OrderKey, Order)>,
) -> StdResult<QueryOrderResponse> {
    let (order_key, order) = res?;
    Ok(into_query_order_response((order_key, order)))
}

fn into_query_order_response(
    ((pair_id, limit_price, timestamp, order_id), order): (OrderKey, Order),
) -> QueryOrderResponse {
    QueryOrderResponse {
        order_id,
        pair_id,
        limit_price,
        timestamp,
        size: order.size,
        reduce_only: order.reduce_only,
        reserved_margin: order.reserved_margin,
    }
}
