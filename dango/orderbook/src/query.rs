use {
    crate::{ORDERS, PAIR},
    dango_types::orderbook::{OrderId, OrderResponse, Pair, QueryMsg},
    grug::{Addr, Bound, ImmutableCtx, Json, JsonSerExt, Order as IterationOrder, StdResult},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Pair {} => {
            let res = query_pair(ctx)?;
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
        QueryMsg::OrdersByTrader {
            trader,
            start_after,
            limit,
        } => {
            let res = query_orders_by_trader(ctx, trader, start_after, limit)?;
            res.to_json_value()
        },
    }
}

#[inline]
fn query_pair(ctx: ImmutableCtx) -> StdResult<Pair> {
    PAIR.load(ctx.storage)
}

#[inline]
fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<OrderResponse> {
    let ((direction, price, _), order) = ORDERS.idx.order_id.load(ctx.storage, order_id)?;

    Ok(OrderResponse {
        direction,
        price,
        trader: order.trader,
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
            let (order_id, (direction, price, _), order) = res?;
            Ok((order_id, OrderResponse {
                direction,
                price,
                trader: order.trader,
                amount: order.amount,
                remaining: order.remaining,
            }))
        })
        .collect()
}

#[inline]
fn query_orders_by_trader(
    _ctx: ImmutableCtx,
    _trader: Addr,
    _start_after: Option<OrderId>,
    _limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrderResponse>> {
    todo!("we need a `.prefix` method in `UniqueIndex` in order to implement this");
}
