use {
    crate::{
        ASKS, BIDS, NEXT_ORDER_ID, NoCachePairQuerier, PAIR_STATES, PARAM, USER_STATES,
        core::{
            accrue_funding, check_minimum_opening, check_oi_constraint, compute_available_margin,
            compute_exec_price, compute_initial_margin, compute_required_margin,
            compute_target_price, compute_trading_fee, compute_user_equity, decompose_fill,
            is_price_constraint_violated,
        },
        execute::ORACLE,
    },
    anyhow::{bail, ensure},
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, UsdPrice,
        perps::{
            Order, OrderId, OrderKind, PairId, PairParam, PairState, Param, UserState,
            settlement_currency,
        },
    },
    grug::{Addr, MutableCtx, NumberConst, QuerierExt, QuerierWrapper, Response, Timestamp},
};

pub fn submit_order(
    ctx: MutableCtx,
    pair_id: PairId,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    let param = PARAM.load(ctx.storage)?;
    let user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    let pair_querier = NoCachePairQuerier::new_local(ctx.storage);
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // --------------------------- 2. Business logic ---------------------------

    let (pair_state, user_state, order_to_store) = _submit_order(
        ctx.sender,
        ctx.block.timestamp,
        ctx.querier,
        &pair_querier,
        &mut oracle_querier,
        &param,
        user_state,
        &pair_id,
        size,
        kind,
        reduce_only,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    PAIR_STATES.save(ctx.storage, &pair_id, &pair_state)?;

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    if let Some((limit_price, order_id, order)) = order_to_store {
        let next_order_id = order_id + OrderId::ONE;
        let order_key = (pair_id, limit_price, ctx.block.timestamp, order_id);

        NEXT_ORDER_ID.save(ctx.storage, &next_order_id)?;

        if size.is_positive() {
            BIDS.save(ctx.storage, order_key, &order)?
        } else {
            ASKS.save(ctx.storage, order_key, &order)?
        };
    }

    Ok(Response::new())
}

/// Returns:
///
/// - The updated `PairState`
/// - The updated `UserState`
/// - GTC order that needs to be stored (if applicable)
fn _submit_order(
    user: Addr,
    current_time: Timestamp,
    querier: QuerierWrapper,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    mut user_state: UserState,
    pair_id: &PairId,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
) -> anyhow::Result<(PairState, UserState, Option<(UsdPrice, OrderId, Order)>)> {
    // ------------- Step 1. Accrue funding before any OI changes --------------

    let pair_param = pair_querier.query_pair_param(&pair_id)?;
    let mut pair_state = pair_querier.query_pair_state(&pair_id)?;

    let oracle_price = oracle_querier.query_price_for_perps(&pair_id)?;

    accrue_funding(&mut pair_state, &pair_param, current_time, oracle_price)?;

    // --------------------------- Step 2. Decompose ---------------------------

    // Find the user's current position in this trading pair.
    let current_position = user_state
        .positions
        .get(&pair_id)
        .map(|position| position.size)
        .unwrap_or_default();

    // Decompose the order into closing and opening portions.
    let (closing_size, mut opening_size) = decompose_fill(size, current_position);

    // Override the opening size with zero if the order is reduce-only.
    if reduce_only {
        opening_size = Quantity::ZERO;
    }

    // This is the size we're going to fill given the reduce-only setting.
    let fillable_size = closing_size.checked_add(opening_size)?;

    ensure!(fillable_size.is_non_zero(), "fillable size is zero");

    // --------------- Step 3. Check minimum opening constraint ----------------

    check_minimum_opening(opening_size, oracle_price, &pair_param)?;

    // ------------------- Step 4. Check OI limit constraint -------------------

    check_oi_constraint(opening_size, &pair_state, &pair_param)?;

    // -------------------- Step 5. Check price constraint ---------------------

    let is_bid = size.is_positive();

    // Compute the target price -- the worst price this order is allowed to be
    // filled at, specified by the user.
    let target_price =
        compute_target_price(kind, oracle_price, pair_state.skew, &pair_param, is_bid)?;

    // Compute the price the order will actually be executed at.
    let exec_price = compute_exec_price(oracle_price, pair_state.skew, fillable_size, &pair_param)?;

    // If execution price is worse than target price:
    // - For market order, abort.
    // - For limit order, save it in the book, then exit.
    if is_price_constraint_violated(exec_price, target_price, is_bid) {
        match kind {
            OrderKind::Market { .. } => {
                bail!(
                    "price exceeds slippage tolerance! execution price: {}, target_price: {}",
                    exec_price,
                    target_price
                );
            },
            OrderKind::Limit { limit_price } => {
                let order_to_store = store_limit_order(
                    user,
                    querier,
                    pair_querier,
                    oracle_querier,
                    param,
                    &pair_param,
                    &mut user_state,
                    size,
                    limit_price,
                    reduce_only,
                )?;

                return Ok((pair_state, user_state, Some(order_to_store)));
            },
        }
    }

    // -------------------- Step 6. Check margin constraint --------------------

    // Compute the user's projected position size after this order is fully filled.
    let projected_size = current_position.checked_add(fillable_size)?;

    // Compute the initial margin requirement after this order is fully filled.
    let initial_margin = compute_initial_margin(
        &user_state,
        pair_querier,
        oracle_querier,
        pair_id,
        projected_size,
    )?;

    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // Compute the trading fee for this order.
    let trading_fee = compute_trading_fee(fillable_size, oracle_price, &param)?;

    // Query the user's collateral balance.
    let collateral_balance = querier.query_balance(user, settlement_currency::DENOM.clone())?;

    // Compute the user's collateral value.
    let collateral_value = Quantity::from_base(collateral_balance, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Compute the user's current equity.
    let equity = compute_user_equity(collateral_value, &user_state, pair_querier, oracle_querier)?;

    // Margin check: user's equity must cover the sum of initial margin, trading
    // fee, and reserved margin from resting limit orders.
    let required_equity = initial_margin
        .checked_add(trading_fee)?
        .checked_add(user_state.reserved_margin)?;

    ensure!(
        equity >= required_equity,
        "insufficient collateral: {} (equity) < {} (initial margin) + {} (trading fee) + {} (reserved margin)",
        equity,
        initial_margin,
        trading_fee,
        user_state.reserved_margin
    );

    // ------------- Step 7. Execute fill and collect trading fee --------------

    execute_fill()?;

    Ok((pair_state, user_state, None))
}

#[inline]
fn store_limit_order(
    user: Addr,
    querier: QuerierWrapper,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    pair_param: &PairParam,
    user_state: &mut UserState,
    size: Quantity,
    mut limit_price: UsdPrice,
    reduce_only: bool,
) -> anyhow::Result<(UsdPrice, OrderId, Order)> {
    ensure!(
        user_state.open_order_count < param.max_open_orders,
        "too many open orders! max allowed: {}",
        param.max_open_orders
    );

    // We now need to reserve some margin from the user's account.
    //
    // This prevents the user from over-committing: either 1) creating a big
    // number of orders or orders of great sizes that his margin can't satisfy,
    // or 2) creating those orders that his margin can satisfy, but withdraw
    // margin later.
    //
    // The reserved margin will be added to the user's `UserState` and used in
    // margin checks when the user places other orders, or when he attempts to
    // withdraw margin. It will be released when the order is fulfilled or canceled.
    //
    // We use the order's limit price to compute the required margin for opening
    // this order. This may not be totally accurate, as when the order is fulfilled
    // later, it may be fulfilled at a different price. This is ok, as when we
    // fill the order, we will do margin check with the actual execution price.
    // For now, we just do a conservative estimation.
    //
    // We technically only need to reserve margin for the order's opening portion.
    // However, we don't know which part of the order will be closing and which
    // will be opening, because that depends on the user's position when the
    // order is fulfilled. Here we assume the worst case, where the entire order
    // is opening.
    let margin_to_reserve = compute_required_margin(size, limit_price, pair_param)?
        .checked_add(compute_trading_fee(size, limit_price, param)?)?;

    // Query the user's current collateral balance.
    let collateral_balance = querier.query_balance(user, settlement_currency::DENOM.clone())?;

    // Query the price of the settlement currency.
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // Compute the USD vlaue of the user's collateral.
    let collateral_value = Quantity::from_base(collateral_balance, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Check how much available margin the user has now, ensure the user has
    // sufficient available margin to cover both.
    let available_margin = compute_available_margin(
        collateral_value,
        user_state,
        pair_querier,
        oracle_querier,
        margin_to_reserve,
    )?;

    ensure!(
        available_margin >= margin_to_reserve,
        "insufficient margin for opening GTC order: {} (available) < {} (required)",
        available_margin,
        margin_to_reserve
    );

    // Update the user state.
    user_state.open_order_count += 1;
    (user_state.reserved_margin).checked_add_assign(margin_to_reserve)?;

    // For buy orders, "invert" the price. This makes it such that the orders
    // are sorted according to price-time priority in the contract storage.
    if size.is_positive() {
        limit_price = UsdPrice::MAX.checked_sub(limit_price)?;
    }

    // Give this order an order ID.
    let order_id = pair_querier.query_next_order_id()?;

    Ok((limit_price, order_id, Order {
        user,
        size,
        reduce_only,
        reserved_margin: margin_to_reserve,
    }))
}

#[inline]
fn execute_fill() -> anyhow::Result<()> {
    Ok(())
}
