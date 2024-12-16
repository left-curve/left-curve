use {
    crate::{ORDERS, PAIRS},
    dango_types::dex::{OrderId, OrderResponse, OrderSide, Pair, PairId, QueryMsg},
    grug::{Bound, ImmutableCtx, Json, JsonSerExt, NumberConst, Order, StdResult, Udec128},
    std::collections::BTreeMap,
};

const DEFAULT_PAGE_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
    match msg {
        QueryMsg::Pair { pair_id } => {
            let res = query_pair(ctx, pair_id)?;
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
    }
}

fn query_pair(ctx: ImmutableCtx, pair_id: PairId) -> StdResult<Pair> {
    PAIRS.load(ctx.storage, pair_id)
}

fn query_pairs(
    ctx: ImmutableCtx,
    start_after: Option<PairId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<PairId, Pair>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    PAIRS
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect()
}

fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<OrderResponse> {
    let ((pair_id, side, limit_price), order) = ORDERS.idx.order_id.load(ctx.storage, order_id)?;
    let limit_price = match (side, limit_price) {
        (OrderSide::Buy, Udec128::MAX) | (OrderSide::Sell, Udec128::MIN) => None,
        _ => Some(limit_price),
    };

    Ok(OrderResponse {
        pair_id,
        side,
        limit_price,
        maker: order.maker,
        size: order.size,
        filled: order.filled,
    })
}

fn query_orders(
    ctx: ImmutableCtx,
    start_after: Option<OrderId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrderResponse>> {
    let start = start_after.map(Bound::Exclusive);
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);

    ORDERS
        .idx
        .order_id
        .range(ctx.storage, start, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| {
            let (order_id, (pair_id, side, limit_price), order) = res?;
            let limit_price = match (side, limit_price) {
                (OrderSide::Buy, Udec128::MAX) | (OrderSide::Sell, Udec128::MIN) => None,
                _ => Some(limit_price),
            };

            Ok((order_id, OrderResponse {
                pair_id,
                side,
                limit_price,
                maker: order.maker,
                size: order.size,
                filled: order.filled,
            }))
        })
        .collect()
}
