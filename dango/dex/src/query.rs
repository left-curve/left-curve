use {
    crate::{ORDERS, PAIRS},
    dango_types::dex::{
        OrderId, OrderResponse, OrdersByPairResponse, OrdersByUserResponse, Pair, PairParams,
        PairUpdate, QueryMsg,
    },
    grug::{
        Addr, Bound, Denom, ImmutableCtx, Json, JsonSerExt, Order as IterationOrder, StdResult,
    },
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Pair {
            base_denom,
            quote_denom,
        } => {
            let res = query_pair(ctx, base_denom, quote_denom)?;
            res.to_json_value()
        },
        QueryMsg::Pairs { start_after, limit } => {
            let res = query_pairs(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::Order { order_id } => {
            let res = query_order(ctx, order_id)?;
            res.to_json_value()
        },
        QueryMsg::Orders { start_after, limit } => {
            let res = query_orders(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::OrdersByPair {
            base_denom,
            quote_denom,
            start_after,
            limit,
        } => {
            let res = query_orders_by_pair(ctx, base_denom, quote_denom, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::OrdersByUser {
            user,
            start_after,
            limit,
        } => {
            let res = query_orders_by_user(ctx, user, start_after, limit)?;
            res.to_json_value()
        },
    }
}

#[inline]
fn query_pair(ctx: ImmutableCtx, base_denom: Denom, quote_denom: Denom) -> StdResult<PairParams> {
    PAIRS.load(ctx.storage, (&base_denom, &quote_denom))
}

#[inline]
fn query_pairs(
    ctx: ImmutableCtx,
    start_after: Option<Pair>,
    limit: Option<u32>,
) -> StdResult<Vec<PairUpdate>> {
    let start = start_after
        .as_ref()
        .map(|p| Bound::Exclusive((&p.base_denom, &p.quote_denom)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    PAIRS
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let ((base_denom, quote_denom), params) = res?;
            Ok(PairUpdate {
                base_denom,
                quote_denom,
                params,
            })
        })
        .collect()
}

#[inline]
fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<OrderResponse> {
    let (((base_denom, quote_denom), direction, price, _), order) =
        ORDERS.idx.order_id.load(ctx.storage, order_id)?;

    Ok(OrderResponse {
        base_denom,
        quote_denom,
        direction,
        price,
        user: order.user,
        amount: order.amount,
        remaining: order.remaining,
    })
}

#[inline]
fn query_orders(
    ctx: ImmutableCtx,
    start_after: Option<OrderId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrderResponse>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ORDERS
        .idx
        .order_id
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let (order_id, ((base_denom, quote_denom), direction, price, _), order) = res?;
            Ok((order_id, OrderResponse {
                base_denom,
                quote_denom,
                direction,
                price,
                user: order.user,
                amount: order.amount,
                remaining: order.remaining,
            }))
        })
        .collect()
}

#[inline]
fn query_orders_by_pair(
    _ctx: ImmutableCtx,
    _base_denom: Denom,
    _quote_denom: Denom,
    _start_after: Option<OrderId>,
    _limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrdersByPairResponse>> {
    todo!();
}

#[inline]
fn query_orders_by_user(
    _ctx: ImmutableCtx,
    _user: Addr,
    _start_after: Option<OrderId>,
    _limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrdersByUserResponse>> {
    todo!();
}
