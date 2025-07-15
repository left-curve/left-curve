use {
    crate::{
        LIMIT_ORDERS, MAX_ORACLE_STALENESS, PAIRS, PassiveLiquidityPool, RESERVES, VOLUMES,
        VOLUMES_BY_USER, core,
    },
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier,
        account_factory::Username,
        dex::{
            OrderId, OrderResponse, OrdersByPairResponse, OrdersByUserResponse, PairId, PairParams,
            PairUpdate, QueryMsg, ReservesResponse, SwapRoute,
        },
    },
    grug::{
        Addr, Bound, Coin, CoinPair, DEFAULT_PAGE_LIMIT, Denom, ImmutableCtx, Inner, Json,
        JsonSerExt, NonZero, Number, NumberConst, Order as IterationOrder, QuerierExt, StdResult,
        Timestamp, Udec128_5, Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> anyhow::Result<Json> {
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
    }
    .map_err(Into::into)
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

fn query_order(ctx: ImmutableCtx, order_id: OrderId) -> StdResult<OrderResponse> {
    let (((base_denom, quote_denom), direction, price, _), order) =
        LIMIT_ORDERS.idx.order_id.load(ctx.storage, order_id)?;

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

    LIMIT_ORDERS
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
            let ((_, direction, price, _), _) =
                LIMIT_ORDERS.idx.order_id.load(ctx.storage, order_id)?;
            Ok(Bound::Exclusive((direction, price, order_id)))
        })
        .transpose()?;
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    LIMIT_ORDERS
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
                LIMIT_ORDERS.idx.order_id.load(ctx.storage, order_id)?;
            Ok(Bound::Exclusive((pair, direction, price, order_id)))
        })
        .transpose()?;
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT) as usize;

    LIMIT_ORDERS
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
fn query_volume(ctx: ImmutableCtx, user: Addr, since: Option<Timestamp>) -> StdResult<Udec128_5> {
    let volume_now = VOLUMES
        .prefix(&user)
        .values(ctx.storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_5::ZERO);

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
            .unwrap_or(Udec128_5::ZERO)
    } else {
        Udec128_5::ZERO
    };

    Ok(volume_now.checked_sub(volume_since)?)
}

fn query_volume_by_user(
    ctx: ImmutableCtx,
    user: Username,
    since: Option<Timestamp>,
) -> StdResult<Udec128_5> {
    let volume_now = VOLUMES_BY_USER
        .prefix(&user)
        .values(ctx.storage, None, None, IterationOrder::Descending)
        .next()
        .transpose()?
        .unwrap_or(Udec128_5::ZERO);

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
            .unwrap_or(Udec128_5::ZERO)
    } else {
        Udec128_5::ZERO
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
