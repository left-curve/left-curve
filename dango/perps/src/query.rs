use {
    crate::{
        ASKS, BIDS, CONDITIONAL_ABOVE, CONDITIONAL_BELOW, DEPTHS, OrderKey, PAIR_PARAMS,
        PAIR_STATES, USER_STATES, VOLUMES, round_to_day,
    },
    anyhow::ensure,
    dango_types::{
        UsdPrice, UsdValue,
        perps::{
            ConditionalOrderId, LiquidityDepth, LiquidityDepthResponse, Order, OrderId, PairId,
            PairParam, PairState, QueryConditionalOrderResponse,
            QueryConditionalOrdersByUserResponse, QueryOrderResponse, QueryOrdersByUserResponse,
            UserState,
        },
    },
    grug::{
        Addr, Bound, DEFAULT_PAGE_LIMIT, ImmutableCtx, Order as IterationOrder, StdResult, Storage,
        Timestamp,
    },
    std::collections::BTreeMap,
};

pub fn query_pair_params(
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

pub fn query_pair_states(
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

pub fn query_user_states(
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
pub fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<Option<QueryOrderResponse>> {
    if let Some(record) = BIDS.idx.order_id.may_load(ctx.storage, order_id)? {
        return Ok(Some(into_query_order_response_with_inverted_price(record)));
    }

    if let Some(record) = ASKS.idx.order_id.may_load(ctx.storage, order_id)? {
        return Ok(Some(into_query_order_response(record)));
    }

    Ok(None)
}

pub fn query_orders_by_user(ctx: ImmutableCtx, user: Addr) -> StdResult<QueryOrdersByUserResponse> {
    let bids = BIDS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(try_into_query_order_response_with_inverted_price)
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

fn into_query_order_response(
    ((pair_id, limit_price, order_id), order): (OrderKey, Order),
) -> QueryOrderResponse {
    QueryOrderResponse {
        order_id,
        pair_id,
        limit_price,
        size: order.size,
        reduce_only: order.reduce_only,
        reserved_margin: order.reserved_margin,
    }
}

/// When storing orders into the `BIDS` map, we "inverted" the price so that
/// orders are sorted respecting the price-time priority.
/// Now, reverse the inversion, so the response contains the original limit price.
fn into_query_order_response_with_inverted_price(
    ((pair_id, limit_price, order_id), order): (OrderKey, Order),
) -> QueryOrderResponse {
    let limit_price = !limit_price;
    into_query_order_response(((pair_id, limit_price, order_id), order))
}

fn try_into_query_order_response(
    res: StdResult<(OrderKey, Order)>,
) -> StdResult<QueryOrderResponse> {
    res.map(into_query_order_response)
}

fn try_into_query_order_response_with_inverted_price(
    res: StdResult<(OrderKey, Order)>,
) -> StdResult<QueryOrderResponse> {
    res.map(into_query_order_response_with_inverted_price)
}

pub fn query_conditional_order(
    ctx: ImmutableCtx,
    order_id: ConditionalOrderId,
) -> StdResult<Option<QueryConditionalOrderResponse>> {
    if let Some(((pair_id, _, order_id), order)) = CONDITIONAL_ABOVE
        .idx
        .order_id
        .may_load(ctx.storage, order_id)?
    {
        return Ok(Some(QueryConditionalOrderResponse {
            order_id,
            pair_id,
            order,
        }));
    }

    if let Some(((pair_id, _, order_id), order)) = CONDITIONAL_BELOW
        .idx
        .order_id
        .may_load(ctx.storage, order_id)?
    {
        return Ok(Some(QueryConditionalOrderResponse {
            order_id,
            pair_id,
            order,
        }));
    }

    Ok(None)
}

pub fn query_conditional_orders_by_user(
    ctx: ImmutableCtx,
    user: Addr,
) -> StdResult<QueryConditionalOrdersByUserResponse> {
    let above = CONDITIONAL_ABOVE
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((pair_id, _, order_id), order) = res?;
            Ok(QueryConditionalOrderResponse {
                order_id,
                pair_id,
                order,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let below = CONDITIONAL_BELOW
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((pair_id, _, order_id), order) = res?;
            Ok(QueryConditionalOrderResponse {
                order_id,
                pair_id,
                order,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(QueryConditionalOrdersByUserResponse { above, below })
}

pub fn query_liquidity_depth(
    ctx: ImmutableCtx,
    pair_id: PairId,
    bucket_size: UsdPrice,
    limit: Option<u32>,
) -> anyhow::Result<LiquidityDepthResponse> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;
    let pair_param = PAIR_PARAMS.load(ctx.storage, &pair_id)?;

    ensure!(
        pair_param.bucket_sizes.contains(&bucket_size),
        "bucket size {bucket_size} not configured for pair {pair_id}"
    );

    let bids = DEPTHS
        .prefix(&pair_id)
        .append(bucket_size)
        .append(true)
        .range(ctx.storage, None, None, IterationOrder::Descending)
        .take(limit)
        .map(|res| {
            let (bucket, (size, notional)) = res?;
            Ok((bucket, LiquidityDepth { size, notional }))
        })
        .collect::<StdResult<_>>()?;

    let asks = DEPTHS
        .prefix(&pair_id)
        .append(bucket_size)
        .append(false)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let (bucket, (size, notional)) = res?;
            Ok((bucket, LiquidityDepth { size, notional }))
        })
        .collect::<StdResult<_>>()?;

    Ok(LiquidityDepthResponse { bids, asks })
}

pub fn query_volume(
    storage: &dyn Storage,
    user: Addr,
    since: Option<Timestamp>,
) -> StdResult<UsdValue> {
    let latest = VOLUMES
        .prefix(user)
        .range(storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .map(|(_, v)| v)
        .unwrap_or(UsdValue::ZERO);

    match since {
        None => Ok(latest),
        Some(ts) => {
            let day = round_to_day(ts);
            let baseline = VOLUMES
                .prefix(user)
                .range(
                    storage,
                    None,
                    Some(Bound::Inclusive(day)),
                    IterationOrder::Descending,
                )
                .next()
                .transpose()?
                .map(|(_, v)| v)
                .unwrap_or(UsdValue::ZERO);
            Ok(latest.checked_sub(baseline)?)
        },
    }
}
