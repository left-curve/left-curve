use {
    crate::{
        DEPTHS, MAX_ORACLE_STALENESS, ORDERS, PAIRS, PAUSED, RESERVES, RESTING_ORDER_BOOK, VOLUMES,
        VOLUMES_BY_USER,
        core::{self, PassiveLiquidityPool},
    },
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::Username,
        dex::{
            Direction, LiquidityDepth, LiquidityDepthResponse, OrderId, OrderResponse,
            OrdersByPairResponse, OrdersByUserResponse, PairId, PairParams, PairUpdate, QueryMsg,
            ReflectCurveResponse, ReservesResponse, RestingOrderBookState,
            RestingOrderBookStatesResponse, SwapRoute,
        },
    },
    grug::{
        Addr, Bound, Coin, CoinPair, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Inner, Json,
        JsonSerExt, NonZero, Number, NumberConst, Order as IterationOrder, QuerierExt, StdResult,
        Timestamp, Udec128_6, Udec128_24, Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
    match msg {
        QueryMsg::Paused {} => {
            let res = query_paused(ctx)?;
            res.to_json_value()
        },
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
        QueryMsg::Reserve {
            base_denom,
            quote_denom,
        } => {
            let res = query_reserve(ctx, base_denom, quote_denom)?;
            res.to_json_value()
        },
        QueryMsg::Reserves { start_after, limit } => {
            let res = query_reserves(ctx, start_after, limit)?;
            res.to_json_value()
        },
        QueryMsg::RestingOrderBookState {
            base_denom,
            quote_denom,
        } => {
            let res = query_resting_order_book_state(ctx, base_denom, quote_denom)?;
            res.to_json_value()
        },
        QueryMsg::RestingOrderBookStates { start_after, limit } => {
            let res = query_resting_order_book_states(ctx, start_after, limit)?;
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
        QueryMsg::Volume { user, since } => {
            let res = query_volume(ctx, user, since)?;
            res.to_json_value()
        },
        QueryMsg::VolumeByUser { user, since } => {
            let res = query_volume_by_user(ctx, user, since)?;
            res.to_json_value()
        },
        QueryMsg::SimulateProvideLiquidity {
            base_denom,
            quote_denom,
            deposit,
        } => {
            let res = query_simulate_provide_liquidity(ctx, base_denom, quote_denom, deposit)?;
            res.to_json_value()
        },
        QueryMsg::SimulateWithdrawLiquidity {
            base_denom,
            quote_denom,
            lp_burn_amount,
        } => {
            let res =
                query_simulate_withdraw_liquidity(ctx, base_denom, quote_denom, lp_burn_amount)?;
            res.to_json_value()
        },
        QueryMsg::SimulateSwapExactAmountIn { route, input } => {
            let res = query_simulate_swap_exact_amount_in(ctx, route, input)?;
            res.to_json_value()
        },
        QueryMsg::SimulateSwapExactAmountOut { route, output } => {
            let res = query_simulate_swap_exact_amount_out(ctx, route, output)?;
            res.to_json_value()
        },
        QueryMsg::ReflectCurve {
            base_denom,
            quote_denom,
            limit,
        } => {
            let res = query_reflect_curve(ctx, base_denom, quote_denom, limit)?;
            res.to_json_value()
        },
        QueryMsg::LiquidityDepth {
            base_denom,
            quote_denom,
            bucket_size,
            limit,
        } => {
            let res = query_liquidity_depth(
                ctx,
                base_denom,
                quote_denom,
                bucket_size,
                limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize,
            )?;
            res.to_json_value()
        },
    }
    .map_err(Into::into)
}

fn query_liquidity_depth(
    ctx: ImmutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    bucket_size: Udec128_24,
    limit: usize,
) -> anyhow::Result<LiquidityDepthResponse> {
    // load the pair params
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    anyhow::ensure!(
        pair.bucket_sizes.contains(&NonZero::new(bucket_size)?),
        "Bucket size {bucket_size} not found for pair ({base_denom}, {quote_denom})"
    );

    // Load the resting order book.
    let resting_order_book = RESTING_ORDER_BOOK.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the liquidity depth for asks.
    let ask_depth = resting_order_book
        .best_ask_price
        .map(|best_ask_price| {
            DEPTHS
                .prefix((&base_denom, &quote_denom))
                .append(bucket_size)
                .append(Direction::Ask)
                .range(
                    ctx.storage,
                    Some(Bound::Inclusive(best_ask_price)),
                    None,
                    IterationOrder::Ascending,
                )
                .take(limit)
                .map(|res| {
                    let (bucket, (depth_base, depth_quote)) = res?;
                    Ok((bucket, LiquidityDepth {
                        depth_base,
                        depth_quote,
                    }))
                })
                .collect::<StdResult<_>>()
        })
        .transpose()?;

    // Load the liquidity depth for bids.
    let bid_depth = resting_order_book
        .best_bid_price
        .map(|best_bid_price| {
            DEPTHS
                .prefix((&base_denom, &quote_denom))
                .append(bucket_size)
                .append(Direction::Bid)
                .range(
                    ctx.storage,
                    None,
                    Some(Bound::Inclusive(best_bid_price)),
                    IterationOrder::Descending,
                )
                .take(limit)
                .map(|res| {
                    let (bucket, (depth_base, depth_quote)) = res?;
                    Ok((bucket, LiquidityDepth {
                        depth_base,
                        depth_quote,
                    }))
                })
                .collect::<StdResult<_>>()
        })
        .transpose()?;

    Ok(LiquidityDepthResponse {
        bid_depth,
        ask_depth,
    })
}

fn query_paused(ctx: ImmutableCtx) -> StdResult<bool> {
    PAUSED.load(ctx.storage)
}

fn query_pair(ctx: ImmutableCtx, base_denom: Denom, quote_denom: Denom) -> StdResult<PairParams> {
    PAIRS.load(ctx.storage, (&base_denom, &quote_denom))
}

fn query_pairs(
    ctx: ImmutableCtx,
    start_after: Option<PairId>,
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

fn query_reserve(ctx: ImmutableCtx, base_denom: Denom, quote_denom: Denom) -> StdResult<CoinPair> {
    RESERVES.load(ctx.storage, (&base_denom, &quote_denom))
}

fn query_reserves(
    ctx: ImmutableCtx,
    start_after: Option<PairId>,
    limit: Option<u32>,
) -> StdResult<Vec<ReservesResponse>> {
    let start = start_after
        .as_ref()
        .map(|pair| Bound::Exclusive((&pair.base_denom, &pair.quote_denom)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    RESERVES
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let ((base_denom, quote_denom), reserve) = res?;
            Ok(ReservesResponse {
                pair: PairId {
                    base_denom,
                    quote_denom,
                },
                reserve,
            })
        })
        .collect()
}

fn query_resting_order_book_state(
    ctx: ImmutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> StdResult<RestingOrderBookState> {
    RESTING_ORDER_BOOK.load(ctx.storage, (&base_denom, &quote_denom))
}

fn query_resting_order_book_states(
    ctx: ImmutableCtx,
    start_after: Option<PairId>,
    limit: Option<u32>,
) -> StdResult<Vec<RestingOrderBookStatesResponse>> {
    let start = start_after
        .as_ref()
        .map(|pair| Bound::Exclusive((&pair.base_denom, &pair.quote_denom)));
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    RESTING_ORDER_BOOK
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let ((base_denom, quote_denom), state) = res?;
            Ok(RestingOrderBookStatesResponse {
                pair: PairId {
                    base_denom,
                    quote_denom,
                },
                state,
            })
        })
        .collect()
}

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

fn query_orders_by_pair(
    ctx: ImmutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    start_after: Option<OrderId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrdersByPairResponse>> {
    let start = start_after
        .map(|order_id| -> StdResult<_> {
            let ((_, direction, price, _), _) = ORDERS.idx.order_id.load(ctx.storage, order_id)?;
            Ok(Bound::Exclusive((direction, price, order_id)))
        })
        .transpose()?;
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ORDERS
        .prefix((base_denom, quote_denom))
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let ((direction, price, order_id), order) = res?;
            Ok((order_id, OrdersByPairResponse {
                user: order.user,
                direction,
                price,
                amount: order.amount,
                remaining: order.remaining,
            }))
        })
        .collect()
}

fn query_orders_by_user(
    ctx: ImmutableCtx,
    user: Addr,
    start_after: Option<OrderId>,
    limit: Option<u32>,
) -> StdResult<BTreeMap<OrderId, OrdersByUserResponse>> {
    let start = start_after
        .map(|order_id| -> StdResult<_> {
            let ((pair, direction, price, _), _) =
                ORDERS.idx.order_id.load(ctx.storage, order_id)?;
            Ok(Bound::Exclusive((pair, direction, price, order_id)))
        })
        .transpose()?;
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    ORDERS
        .idx
        .user
        .prefix(user)
        .range(ctx.storage, start, None, IterationOrder::Ascending)
        .take(limit)
        .map(|res| {
            let (((base_denom, quote_denom), direction, price, order_id), order) = res?;
            Ok((order_id, OrdersByUserResponse {
                base_denom,
                quote_denom,
                direction,
                price,
                amount: order.amount,
                remaining: order.remaining,
            }))
        })
        .collect()
}

#[inline]
fn query_volume(ctx: ImmutableCtx, user: Addr, since: Option<Timestamp>) -> StdResult<Udec128_6> {
    let volume_now = VOLUMES
        .prefix(&user)
        .values(ctx.storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_6::ZERO);

    let volume_since = if let Some(since) = since {
        VOLUMES
            .prefix(&user)
            .values(
                ctx.storage,
                None,
                Some(Bound::Inclusive(since)),
                IterationOrder::Descending,
            )
            .next()
            .transpose()?
            .unwrap_or(Udec128_6::ZERO)
    } else {
        Udec128_6::ZERO
    };

    Ok(volume_now.checked_sub(volume_since)?)
}

fn query_volume_by_user(
    ctx: ImmutableCtx,
    user: Username,
    since: Option<Timestamp>,
) -> StdResult<Udec128_6> {
    let volume_now = VOLUMES_BY_USER
        .prefix(&user)
        .values(ctx.storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_6::ZERO);

    let volume_since = if let Some(since) = since {
        VOLUMES_BY_USER
            .prefix(&user)
            .values(
                ctx.storage,
                None,
                Some(Bound::Inclusive(since)),
                IterationOrder::Descending,
            )
            .next()
            .transpose()?
            .unwrap_or(Udec128_6::ZERO)
    } else {
        Udec128_6::ZERO
    };

    Ok(volume_now.checked_sub(volume_since)?)
}

fn query_simulate_provide_liquidity(
    ctx: ImmutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    deposit: CoinPair,
) -> anyhow::Result<Coin> {
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier);
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;
    let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    pair.add_liquidity(&mut oracle_querier, reserve, lp_token_supply, deposit)
        .map(|(_, lp_mint_amount)| Coin {
            denom: pair.lp_denom,
            amount: lp_mint_amount,
        })
}

fn query_simulate_withdraw_liquidity(
    ctx: ImmutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    lp_burn_amount: Uint128,
) -> anyhow::Result<CoinPair> {
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;
    let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    pair.remove_liquidity(reserve, lp_token_supply, lp_burn_amount)
        .map(|(_, underlying_refund_amount)| underlying_refund_amount)
}

fn query_simulate_swap_exact_amount_in(
    ctx: ImmutableCtx,
    route: SwapRoute,
    input: Coin,
) -> anyhow::Result<Coin> {
    let app_cfg = ctx.querier.query_dango_config()?;
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    core::swap_exact_amount_in(
        ctx.storage,
        &mut oracle_querier,
        *app_cfg.taker_fee_rate,
        route.into_inner(),
        input,
    )
    .map(|(_, output, _)| output)
}

fn query_simulate_swap_exact_amount_out(
    ctx: ImmutableCtx,
    route: SwapRoute,
    output: NonZero<Coin>,
) -> anyhow::Result<Coin> {
    let app_cfg = ctx.querier.query_dango_config()?;
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    core::swap_exact_amount_out(
        ctx.storage,
        &mut oracle_querier,
        *app_cfg.taker_fee_rate,
        route.into_inner(),
        output,
    )
    .map(|(_, input, _)| input)
}

fn query_reflect_curve(
    ctx: ImmutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    limit: Option<u32>,
) -> anyhow::Result<ReflectCurveResponse> {
    // Create oracle querier.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Load the pool's params and reserve.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;
    let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Reflect the curve.
    let (bids, asks) =
        pair.reflect_curve(&mut oracle_querier, base_denom, quote_denom, &reserve)?;

    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    Ok(ReflectCurveResponse {
        bids: bids.take(limit).collect(),
        asks: asks.take(limit).collect(),
    })
}
