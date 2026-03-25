use {
    crate::{
        ASKS, BIDS, CONDITIONAL_ABOVE, CONDITIONAL_BELOW, DEPTHS, FEE_SHARE_RATIO, PAIR_PARAMS,
        PAIR_STATES, REFEREE_TO_REFERRER, USER_STATES, VOLUMES, round_to_day,
    },
    anyhow::ensure,
    dango_types::{
        UsdPrice, UsdValue,
        account_factory::UserIndex,
        perps::{
            ConditionalOrder, FeeShareRatio, LimitOrConditionalOrder, LimitOrder, LiquidityDepth,
            LiquidityDepthResponse, OrderId, PairId, PairParam, PairState, QueryOrderResponse,
            QueryOrdersByUserResponseItem, Referrer, UserState,
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

/// Search all 4 order maps (`BIDS`, `ASKS`, `CONDITIONAL_ABOVE`, `CONDITIONAL_BELOW`)
/// for an order with the given ID. Since `OrderId` and `ConditionalOrderId` share
/// the same ID space, an ID appears in exactly one map.
pub fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<Option<QueryOrderResponse>> {
    // Check `BIDS` (un-invert price).
    if let Some(((pair_id, stored_price, _), order)) =
        BIDS.idx.order_id.may_load(ctx.storage, order_id)?
    {
        return Ok(Some(limit_order_to_response(pair_id, !stored_price, order)));
    }

    // Check `ASKS` (price as-is).
    if let Some(((pair_id, limit_price, _), order)) =
        ASKS.idx.order_id.may_load(ctx.storage, order_id)?
    {
        return Ok(Some(limit_order_to_response(pair_id, limit_price, order)));
    }

    // Check `CONDITIONAL_ABOVE`.
    if let Some(((pair_id, ..), order)) = CONDITIONAL_ABOVE
        .idx
        .order_id
        .may_load(ctx.storage, order_id)?
    {
        return Ok(Some(conditional_order_to_response(pair_id, order)));
    }

    // Check `CONDITIONAL_BELOW`.
    if let Some(((pair_id, ..), order)) = CONDITIONAL_BELOW
        .idx
        .order_id
        .may_load(ctx.storage, order_id)?
    {
        return Ok(Some(conditional_order_to_response(pair_id, order)));
    }

    Ok(None)
}

/// Return all orders (limit + conditional) for a user, keyed by order ID.
pub fn query_orders_by_user(
    ctx: ImmutableCtx,
    user: Addr,
) -> StdResult<BTreeMap<OrderId, QueryOrdersByUserResponseItem>> {
    let mut items = BTreeMap::new();

    // `BIDS` (un-invert price).
    for res in BIDS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
    {
        let ((pair_id, stored_price, order_id), order) = res?;
        items.insert(order_id, limit_order_to_item(pair_id, !stored_price, order));
    }

    // `ASKS` (price as-is).
    for res in ASKS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, None, None, IterationOrder::Ascending)
    {
        let ((pair_id, limit_price, order_id), order) = res?;
        items.insert(order_id, limit_order_to_item(pair_id, limit_price, order));
    }

    // `CONDITIONAL_ABOVE`.
    for res in CONDITIONAL_ABOVE.idx.user.prefix(user).range(
        ctx.storage,
        None,
        None,
        IterationOrder::Ascending,
    ) {
        let ((pair_id, _, order_id), order) = res?;
        items.insert(order_id, conditional_order_to_item(pair_id, order));
    }

    // `CONDITIONAL_BELOW`.
    for res in CONDITIONAL_BELOW.idx.user.prefix(user).range(
        ctx.storage,
        None,
        None,
        IterationOrder::Ascending,
    ) {
        let ((pair_id, _, order_id), order) = res?;
        items.insert(order_id, conditional_order_to_item(pair_id, order));
    }

    Ok(items)
}

fn limit_order_to_response(
    pair_id: PairId,
    limit_price: UsdPrice,
    order: LimitOrder,
) -> QueryOrderResponse {
    QueryOrderResponse {
        user: order.user,
        pair_id,
        size: order.size,
        kind: LimitOrConditionalOrder::Limit {
            limit_price,
            reduce_only: order.reduce_only,
            reserved_margin: order.reserved_margin,
        },
        created_at: order.created_at,
    }
}

fn conditional_order_to_response(pair_id: PairId, order: ConditionalOrder) -> QueryOrderResponse {
    QueryOrderResponse {
        user: order.user,
        pair_id,
        size: order.size,
        kind: LimitOrConditionalOrder::Conditional {
            trigger_price: order.trigger_price,
            trigger_direction: order.trigger_direction,
        },
        created_at: order.created_at,
    }
}

fn limit_order_to_item(
    pair_id: PairId,
    limit_price: UsdPrice,
    order: LimitOrder,
) -> QueryOrdersByUserResponseItem {
    QueryOrdersByUserResponseItem {
        pair_id,
        size: order.size,
        kind: LimitOrConditionalOrder::Limit {
            limit_price,
            reduce_only: order.reduce_only,
            reserved_margin: order.reserved_margin,
        },
        created_at: order.created_at,
    }
}

fn conditional_order_to_item(
    pair_id: PairId,
    order: ConditionalOrder,
) -> QueryOrdersByUserResponseItem {
    QueryOrdersByUserResponseItem {
        pair_id,
        size: order.size,
        kind: LimitOrConditionalOrder::Conditional {
            trigger_price: order.trigger_price,
            trigger_direction: order.trigger_direction,
        },
        created_at: order.created_at,
    }
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

pub fn query_referrer(storage: &dyn Storage, referee: UserIndex) -> StdResult<Option<Referrer>> {
    REFEREE_TO_REFERRER.may_load(storage, referee)
}

pub fn query_fee_share_ratio(
    storage: &dyn Storage,
    referrer: UserIndex,
) -> StdResult<Option<FeeShareRatio>> {
    FEE_SHARE_RATIO.may_load(storage, referrer)
}
