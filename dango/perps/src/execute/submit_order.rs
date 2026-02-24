use {
    crate::{
        ASKS, BIDS, NEXT_ORDER_ID, NoCachePairQuerier, PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{
            accrue_funding, check_minimum_opening, check_oi_constraint, compute_available_margin,
            compute_exec_price, compute_initial_margin, compute_position_unrealized_funding,
            compute_required_margin, compute_target_price, compute_trading_fee,
            compute_user_equity, decompose_fill, is_price_constraint_violated,
        },
        execute::{BANK, ORACLE},
    },
    anyhow::{bail, ensure},
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, UsdPrice, UsdValue, bank,
        perps::{
            Order, OrderId, OrderKind, PairId, PairParam, PairState, Param, Position, State,
            UserState, settlement_currency,
        },
    },
    grug::{
        Addr, Coins, IsZero, MathResult, Message, MutableCtx, Number, NumberConst, QuerierExt,
        QuerierWrapper, Response, Timestamp, Uint128, coins,
    },
    std::cmp::Ordering,
};

pub fn submit_order(
    ctx: MutableCtx,
    pair_id: PairId,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    let state = STATE.load(ctx.storage)?;
    let param = PARAM.load(ctx.storage)?;
    let user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    let pair_querier = NoCachePairQuerier::new_local(ctx.storage);
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // --------------------------- 2. Business logic ---------------------------

    let (state, pair_state, user_state, vault_pays_user, user_pays_vault, order_to_store) =
        _submit_order(
            ctx.sender,
            ctx.block.timestamp,
            ctx.querier,
            &pair_querier,
            &mut oracle_querier,
            &param,
            state,
            user_state,
            &pair_id,
            size,
            kind,
            reduce_only,
        )?;

    // ------------------------ 3. Apply state changes -------------------------

    STATE.save(ctx.storage, &state)?;

    PAIR_STATES.save(ctx.storage, &pair_id, &pair_state)?;

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    if let Some((limit_price, order_id, order)) = order_to_store {
        // Increment the order ID.
        let next_order_id = order_id + OrderId::ONE;
        let order_key = (pair_id, limit_price, ctx.block.timestamp, order_id);

        NEXT_ORDER_ID.save(ctx.storage, &next_order_id)?;

        if size.is_positive() {
            BIDS.save(ctx.storage, order_key, &order)?
        } else {
            ASKS.save(ctx.storage, order_key, &order)?
        };
    }

    Ok(Response::new()
        .may_add_message(if vault_pays_user.is_non_zero() {
            Some(Message::transfer(
                ctx.sender,
                coins! { settlement_currency::DENOM.clone() => vault_pays_user },
            )?)
        } else {
            None
        })
        .may_add_message(if user_pays_vault.is_non_zero() {
            Some(Message::execute(
                BANK,
                &bank::ExecuteMsg::ForceTransfer {
                    from: ctx.sender,
                    to: ctx.contract,
                    coins: coins! { settlement_currency::DENOM.clone() => user_pays_vault },
                },
                Coins::new(),
            )?)
        } else {
            None
        }))
}

/// Returns:
///
/// - The updated `State`
/// - The updated `PairState`
/// - The updated `UserState`
/// - The amount of settlement currency that the vault needs to pay the user
///   (in case there is a positive after-fee realized PnL)
/// - The amount of settlement currency that the user needs to pay the vault
///   (in case there is a negative after-fee realized PnL)
/// - GTC order that needs to be stored (if applicable)
fn _submit_order(
    user: Addr,
    current_time: Timestamp,
    querier: QuerierWrapper,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    mut state: State,
    mut user_state: UserState,
    pair_id: &PairId,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
) -> anyhow::Result<(
    State,
    PairState,
    UserState,
    Uint128,
    Uint128,
    Option<(UsdPrice, OrderId, Order)>,
)> {
    // ------------- Step 1. Accrue funding before any OI changes --------------

    let pair_param = pair_querier.query_pair_param(pair_id)?;
    let mut pair_state = pair_querier.query_pair_state(pair_id)?;

    let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;

    accrue_funding(&mut pair_state, &pair_param, current_time, oracle_price)?;

    // --------------------------- Step 2. Decompose ---------------------------

    // Find the user's current position in this trading pair.
    let current_position = user_state
        .positions
        .get(pair_id)
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
                    "slippage exceeds tolerance! execution price: {}, target_price: {}",
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

                return Ok((
                    state,
                    pair_state,
                    user_state,
                    Uint128::ZERO,
                    Uint128::ZERO,
                    Some(order_to_store),
                ));
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
    let trading_fee = compute_trading_fee(fillable_size, exec_price, param)?;

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

    // ----------------------- Step 7. Execute the fill ------------------------

    let pnl = execute_fill(
        &mut pair_state,
        &mut user_state,
        pair_id,
        exec_price,
        closing_size,
        opening_size,
    )?;

    let (vault_pays_user, user_pays_vault) =
        settle_pnl_and_fee(&mut state, pnl, trading_fee, settlement_currency_price)?;

    Ok((
        state,
        pair_state,
        user_state,
        vault_pays_user,
        user_pays_vault,
        None,
    ))
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

/// Fill an order. Update pair and user states, return the PnL and needs to be
/// realized (without considering trading fee, which is to be handled by the
/// caller function).
#[inline]
fn execute_fill(
    pair_state: &mut PairState,
    user_state: &mut UserState,
    pair_id: &PairId,
    exec_price: UsdPrice,
    closing_size: Quantity,
    opening_size: Quantity,
) -> MathResult<UsdValue> {
    let mut pnl = UsdValue::ZERO;

    // If a position already exists, remove its contributions to the accumulators
    // and settle funding.
    if let Some(position) = user_state.positions.get_mut(pair_id) {
        (pair_state.oi_weighted_entry_price)
            .checked_sub_assign(position.size.checked_mul(position.entry_price)?)?;
        (pair_state.oi_weighted_entry_funding)
            .checked_sub_assign(position.size.checked_mul(position.entry_funding_per_unit)?)?;

        let funding_pnl = settle_funding(position, pair_state)?;
        pnl = pnl.checked_add(funding_pnl)?;
    }

    // Execute the closing portion of the order. Compute realized PnL.
    if closing_size.is_non_zero() {
        let closing_pnl = apply_closing(user_state, pair_id, closing_size, exec_price)?;
        pnl = pnl.checked_add(closing_pnl)?;
    }

    // Execute the opening portion of the order.
    if opening_size.is_non_zero() {
        apply_opening(user_state, pair_state, pair_id, opening_size, exec_price)?;
    }

    // Re-add accumulator contributions of the updated position.
    if let Some(position) = user_state.positions.get_mut(pair_id) {
        (pair_state.oi_weighted_entry_price)
            .checked_add_assign(position.size.checked_mul(position.entry_price)?)?;
        (pair_state.oi_weighted_entry_funding)
            .checked_add_assign(position.size.checked_mul(position.entry_funding_per_unit)?)?;
    }

    // Update open interest.
    update_oi(pair_state, closing_size, opening_size)?;

    Ok(pnl)
}

/// Settle funding accrued on a position since it was last touched.
///
/// Resets the position's funding entry point to the current cumulative value.
/// Returns the PnL from the user's perspective (negated accrued funding,
/// since positive accrued = user cost).
#[inline]
fn settle_funding(position: &mut Position, pair_state: &PairState) -> MathResult<UsdValue> {
    let accrued = compute_position_unrealized_funding(position, pair_state)?;

    position.entry_funding_per_unit = pair_state.funding_per_unit;

    accrued.checked_neg()
}

/// Close a portion of an existing position: realize PnL and reduce size.
///
/// Removes the position entirely if fully closed.
#[inline]
fn apply_closing(
    user_state: &mut UserState,
    pair_id: &PairId,
    closing_size: Quantity,
    exec_price: UsdPrice,
) -> MathResult<UsdValue> {
    let position = user_state.positions.get_mut(pair_id).unwrap();

    let pnl = compute_pnl_to_realize(position, closing_size, exec_price)?;

    position.size.checked_add_assign(closing_size)?;

    if position.size.is_zero() {
        user_state.positions.remove(pair_id);
    }

    Ok(pnl)
}

/// Grow an existing position or create a new one.
///
/// For existing positions, blends the entry price as a weighted average.
/// For new positions (or positions fully closed then reopened), sets
/// the entry price and funding entry point directly.
#[inline]
fn apply_opening(
    user_state: &mut UserState,
    pair_state: &PairState,
    pair_id: &PairId,
    opening_size: Quantity,
    exec_price: UsdPrice,
) -> MathResult<()> {
    if let Some(position) = user_state.positions.get_mut(pair_id) {
        let old_size = position.size;
        position.size.checked_add_assign(opening_size)?;

        if old_size.is_zero() {
            // Fully closed by `apply_closing`, now reopening opposite side.
            position.entry_price = exec_price;
            position.entry_funding_per_unit = pair_state.funding_per_unit;
        } else {
            // Weighted average entry price.
            let old_notional = old_size.checked_abs()?.checked_mul(position.entry_price)?;
            let new_notional = opening_size.checked_abs()?.checked_mul(exec_price)?;

            position.entry_price = old_notional
                .checked_add(new_notional)?
                .checked_div(position.size.checked_abs()?)?;
        }
    } else {
        user_state.positions.insert(pair_id.clone(), Position {
            size: opening_size,
            entry_price: exec_price,
            entry_funding_per_unit: pair_state.funding_per_unit,
        });
    }

    Ok(())
}

/// Compute the PnL to be realized when closing a portion of a position.
///
/// - Long positions: profit when exit > entry
/// - Short positions: profit when entry > exit
#[inline]
fn compute_pnl_to_realize(
    position: &Position,
    closing_size: Quantity,
    exec_price: UsdPrice,
) -> MathResult<UsdValue> {
    let entry_value = closing_size
        .checked_abs()?
        .checked_mul(position.entry_price)?;
    let exit_value = closing_size.checked_abs()?.checked_mul(exec_price)?;

    if position.size.is_positive() {
        Ok(exit_value.checked_sub(entry_value)?)
    } else {
        Ok(entry_value.checked_sub(exit_value)?)
    }
}

#[inline]
fn update_oi(
    pair_state: &mut PairState,
    closing_size: Quantity,
    opening_size: Quantity,
) -> MathResult<()> {
    if closing_size.is_negative() {
        // Cloing a long position with a sell order.
        pair_state.long_oi.checked_add_assign(closing_size)?;
    } else if closing_size.is_positive() {
        // Closing a short position with a buy order.
        pair_state.short_oi.checked_sub_assign(closing_size)?;
    }

    if opening_size.is_positive() {
        // Open a long position with a buy order.
        pair_state.long_oi.checked_add_assign(opening_size)?;
    } else if opening_size.is_negative() {
        // Open a short position with a sell order.
        (pair_state.short_oi).checked_add_assign(opening_size.checked_abs()?)?;
    }

    Ok(())
}

/// Settle PnL and trading fee between the vault and the user.
///
/// Returns a tuple of two values:
///
/// - The amount of settlement currency that the vault pays the user.
/// - The amount of settlement currency that the user pays the vault.
#[inline]
fn settle_pnl_and_fee(
    state: &mut State,
    pnl: UsdValue,
    trading_fee: UsdValue,
    settlement_currency_price: UsdPrice,
) -> MathResult<(Uint128, Uint128)> {
    // 1. Subtract trading fee from PnL.
    // 2. Convert PnL from USD value from USD value to quantity of settlement currency.
    let pnl = pnl
        .checked_sub(trading_fee)?
        .checked_div(settlement_currency_price)?;

    match pnl.cmp(&Quantity::ZERO) {
        // PnL minus fee is positive: vault pays the user.
        // Round the number down, to the advantage of the protocol and disadvantage of the user.
        Ordering::Greater => {
            let pnl = pnl.into_base_floor(settlement_currency::DECIMAL)?;

            state.vault_margin.checked_sub_assign(pnl)?;

            Ok((pnl, Uint128::ZERO))
        },
        // PnL minus fee is negative: user pays the vault.
        // Round the up, following the same rounding principle.
        Ordering::Less => {
            let pnl = (pnl.checked_abs()?).into_base_ceil(settlement_currency::DECIMAL)?;

            state.vault_margin.checked_add_assign(pnl)?;

            Ok((Uint128::ZERO, pnl))
        },
        // PnL minus is zero. This is an edge case: there must be a positive PnL
        // that perfectly cancels out trading fee.
        Ordering::Equal => Ok((Uint128::ZERO, Uint128::ZERO)),
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{Dimensionless, FundingPerUnit, constants::eth, oracle::PrecisionedPrice},
        grug::{MockQuerier, Udec128, hash_map},
    };

    // -------------------------------- helpers --------------------------------

    fn usdc_price() -> PrecisionedPrice {
        PrecisionedPrice::new(Udec128::new_percent(100), Timestamp::from_seconds(0), 6)
    }

    fn eth_price(dollars: u128) -> PrecisionedPrice {
        PrecisionedPrice::new(
            Udec128::new_percent(dollars * 100),
            Timestamp::from_seconds(0),
            18,
        )
    }

    /// Large `skew_scale` so `exec_price == oracle_price`; 5 % initial margin.
    fn test_pair_param() -> PairParam {
        PairParam {
            skew_scale: Quantity::new_int(1_000_000_000),
            max_abs_premium: Dimensionless::new_permille(50),
            max_abs_oi: Quantity::new_int(1_000_000),
            initial_margin_ratio: Dimensionless::new_permille(50), // 5 %
            maintenance_margin_ratio: Dimensionless::new_permille(30), // 3 %
            ..Default::default()
        }
    }

    /// Zero trading fee, 10 max open orders.
    fn test_param() -> Param {
        Param {
            trading_fee_rate: Dimensionless::ZERO,
            max_open_orders: 10,
            ..Default::default()
        }
    }

    /// Market order with a generous 10 % slippage tolerance.
    fn market_order() -> OrderKind {
        OrderKind::Market {
            max_slippage: Dimensionless::new_permille(100),
        }
    }

    fn setup_queriers(
        eth_dollars: u128,
        pair_param: PairParam,
        pair_state: PairState,
        collateral: u128,
    ) -> (
        MockQuerier,
        NoCachePairQuerier<'static>,
        OracleQuerier<'static>,
    ) {
        setup_queriers_ext(eth_dollars, pair_param, pair_state, collateral, None)
    }

    fn setup_queriers_ext(
        eth_dollars: u128,
        pair_param: PairParam,
        pair_state: PairState,
        collateral: u128,
        next_order_id: Option<OrderId>,
    ) -> (
        MockQuerier,
        NoCachePairQuerier<'static>,
        OracleQuerier<'static>,
    ) {
        let mock_q = MockQuerier::new()
            .with_balance(
                Addr::mock(1),
                settlement_currency::DENOM.clone(),
                collateral,
            )
            .unwrap();
        let pair_q = NoCachePairQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => pair_param },
            hash_map! { eth::DENOM.clone() => pair_state },
            next_order_id,
        );
        let oracle_q = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price(),
            eth::DENOM.clone() => eth_price(eth_dollars),
        });
        (mock_q, pair_q, oracle_q)
    }

    /// 100 k USDC in base units (precision 6).
    const COLLATERAL: u128 = 100_000_000_000;
    /// 1 M USDC in base units.
    const VAULT_MARGIN: u128 = 1_000_000_000_000;

    fn default_state() -> State {
        State {
            vault_margin: Uint128::new(VAULT_MARGIN),
            ..Default::default()
        }
    }

    fn make_position(size: i128, entry_price: i128) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        }
    }

    fn position_with_funding(size: i128, entry_price: i128, entry_funding: i128) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::new_int(entry_funding),
        }
    }

    // ======================= Group 1: Early rejections =======================

    #[test]
    fn reduce_only_no_position_errors() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(10),
            market_order(),
            true,
        )
        .unwrap_err();

        assert!(err.to_string().contains("fillable size is zero"));
    }

    #[test]
    fn reduce_only_same_direction_errors() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(5),
                oi_weighted_entry_price: UsdValue::new_int(5_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(5, 1000));

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(10), // same direction as long
            market_order(),
            true,
        )
        .unwrap_err();

        assert!(err.to_string().contains("fillable size is zero"));
    }

    #[test]
    fn reduce_only_partial_close_succeeds() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(5),
                oi_weighted_entry_price: UsdValue::new_int(5_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(5, 1000));

        let (_state, _pair_state, user_state, vault_pays_user, user_pays_vault, order) =
            _submit_order(
                Addr::mock(1),
                Timestamp::from_seconds(0),
                querier,
                &pair_q,
                &mut oracle_q,
                &test_param(),
                default_state(),
                user_state,
                &eth::DENOM,
                Quantity::new_int(-10), // sell 10, only 5 fillable
                market_order(),
                true,
            )
            .unwrap();

        assert!(user_state.positions.is_empty());
        assert!(order.is_none());
        // Profit: 5 * ($2000 - $1000) = $5000
        assert!(vault_pays_user > Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn minimum_opening_violation() {
        let pair_param = PairParam {
            min_opening_size: UsdValue::new_int(10_000),
            ..test_pair_param()
        };

        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            pair_param,
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(1), // notional = 1 * $2000 = $2000 < $10000
            market_order(),
            false,
        )
        .unwrap_err();

        assert!(err.to_string().contains("opening size is below minimum"));
    }

    #[test]
    fn oi_constraint_violation() {
        let pair_param = PairParam {
            max_abs_oi: Quantity::new_int(100),
            ..test_pair_param()
        };

        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            pair_param,
            PairState {
                long_oi: Quantity::new_int(95),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(10), // long_oi = 95 + 10 = 105 > 100
            market_order(),
            false,
        )
        .unwrap_err();

        assert!(err.to_string().contains("max long OI exceeded"));
    }

    // ======================= Group 2: Price constraint =======================

    #[test]
    fn market_order_slippage_exceeded() {
        // Small skew_scale amplifies price impact.
        let pair_param = PairParam {
            skew_scale: Quantity::new_int(100),
            max_abs_premium: Dimensionless::new_permille(500), // 50 %
            ..test_pair_param()
        };

        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            pair_param,
            PairState {
                skew: Quantity::new_int(10),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::ZERO,
            },
            false,
        )
        .unwrap_err();

        assert!(err.to_string().contains("slippage exceeds tolerance"));
    }

    #[test]
    fn limit_buy_stored_on_price_violation() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers_ext(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
            Some(OrderId::ZERO),
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, _pair_state, user_state, vault_pays_user, user_pays_vault, order) =
            _submit_order(
                Addr::mock(1),
                Timestamp::from_seconds(0),
                querier,
                &pair_q,
                &mut oracle_q,
                &test_param(),
                default_state(),
                UserState::default(),
                &eth::DENOM,
                Quantity::new_int(5),
                OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1000), // below oracle $2000
                },
                false,
            )
            .unwrap();

        let (stored_price, order_id, stored_order) = order.unwrap();

        // Buy: limit price is inverted for storage ordering.
        assert_eq!(
            stored_price,
            UsdPrice::MAX.checked_sub(UsdPrice::new_int(1000)).unwrap()
        );
        assert_eq!(order_id, OrderId::ZERO);
        assert_eq!(stored_order.size, Quantity::new_int(5));
        assert!(!stored_order.reduce_only);
        assert_eq!(user_state.open_order_count, 1);
        // reserved_margin = |5| * $1000 * 5 % = $250
        assert_eq!(user_state.reserved_margin, UsdValue::new_int(250));
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn limit_sell_stored_without_inversion() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers_ext(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
            Some(OrderId::ZERO),
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, _pair_state, user_state, vault_pays_user, user_pays_vault, order) =
            _submit_order(
                Addr::mock(1),
                Timestamp::from_seconds(0),
                querier,
                &pair_q,
                &mut oracle_q,
                &test_param(),
                default_state(),
                UserState::default(),
                &eth::DENOM,
                Quantity::new_int(-5),
                OrderKind::Limit {
                    limit_price: UsdPrice::new_int(3000), // above oracle $2000
                },
                false,
            )
            .unwrap();

        let (stored_price, _order_id, stored_order) = order.unwrap();

        // Sell: limit price is NOT inverted.
        assert_eq!(stored_price, UsdPrice::new_int(3000));
        assert_eq!(stored_order.size, Quantity::new_int(-5));
        assert_eq!(user_state.open_order_count, 1);
        // reserved_margin = |-5| * $3000 * 5 % = $750
        assert_eq!(user_state.reserved_margin, UsdValue::new_int(750));
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn limit_order_too_many_open_orders() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers_ext(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
            Some(OrderId::ZERO),
        );
        let querier = QuerierWrapper::new(&mock_q);

        let user_state = UserState {
            open_order_count: 10, // at max
            ..Default::default()
        };

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(5),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(1000),
            },
            false,
        )
        .unwrap_err();

        assert!(err.to_string().contains("too many open orders"));
    }

    #[test]
    fn limit_order_insufficient_margin() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers_ext(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            1_000_000, // only 1 USDC
            Some(OrderId::ZERO),
        );
        let querier = QuerierWrapper::new(&mock_q);

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(100),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(1000),
            },
            false,
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("insufficient margin for opening GTC order")
        );
    }

    // ========================= Group 3: Margin check =========================

    #[test]
    fn insufficient_collateral_errors() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            1_000_000, // 1 USDC
        );
        let querier = QuerierWrapper::new(&mock_q);

        let err = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(100), // margin = 100 * $2000 * 5 % = $10000
            market_order(),
            false,
        )
        .unwrap_err();

        assert!(err.to_string().contains("insufficient collateral"));
    }

    #[test]
    fn margin_check_passes_with_sufficient_collateral() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, _pair_state, user_state, _vpay, _upay, order) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(1), // margin = 1 * $2000 * 5 % = $100
            market_order(),
            false,
        )
        .unwrap();

        assert!(user_state.positions.contains_key(&eth::DENOM));
        assert!(order.is_none());
    }

    // ====================== Group 4: Closing positions =======================

    #[test]
    fn close_long_at_profit() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 1000));

        let (state, _pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        assert!(user_state.positions.is_empty());
        // PnL = 10 * ($2000 - $1000) = $10,000
        assert_eq!(vault_pays_user, Uint128::new(10_000_000_000));
        assert_eq!(user_pays_vault, Uint128::ZERO);
        assert_eq!(
            state.vault_margin,
            Uint128::new(VAULT_MARGIN - 10_000_000_000)
        );
    }

    #[test]
    fn close_long_at_loss() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            1000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(20_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 2000));

        let (state, _pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        assert!(user_state.positions.is_empty());
        // PnL = 10 * ($1000 - $2000) = -$10,000
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::new(10_000_000_000));
        assert_eq!(
            state.vault_margin,
            Uint128::new(VAULT_MARGIN + 10_000_000_000)
        );
    }

    #[test]
    fn close_short_at_profit() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            1000,
            test_pair_param(),
            PairState {
                short_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(-20_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(-10, 2000));

        let (_state, _pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(10), // buy to close short
            market_order(),
            false,
        )
        .unwrap();

        assert!(user_state.positions.is_empty());
        // Short PnL: entry - exit = $2000 - $1000 = $10,000
        assert_eq!(vault_pays_user, Uint128::new(10_000_000_000));
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn close_short_at_loss() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                short_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(-10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(-10, 1000));

        let (_state, _pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(10),
            market_order(),
            false,
        )
        .unwrap();

        assert!(user_state.positions.is_empty());
        // Short PnL: entry - exit = $1000 - $2000 = -$10,000
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::new(10_000_000_000));
    }

    #[test]
    fn partial_close_keeps_position() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 1000));

        let (_state, _pair_state, user_state, vault_pays_user, _upay, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-3), // close 3 of 10
            market_order(),
            false,
        )
        .unwrap();

        let position = user_state.positions.get(&eth::DENOM).unwrap();
        assert_eq!(position.size, Quantity::new_int(7));
        assert_eq!(position.entry_price, UsdPrice::new_int(1000)); // unchanged
        assert!(vault_pays_user > Uint128::ZERO);
    }

    // ====================== Group 5: Opening positions ========================

    #[test]
    fn open_new_long() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(10),
            market_order(),
            false,
        )
        .unwrap();

        let position = user_state.positions.get(&eth::DENOM).unwrap();
        assert_eq!(position.size, Quantity::new_int(10));
        assert_eq!(position.entry_price, UsdPrice::new_int(2000));
        assert_eq!(pair_state.long_oi, Quantity::new_int(10));
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn open_new_short() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        let position = user_state.positions.get(&eth::DENOM).unwrap();
        assert_eq!(position.size, Quantity::new_int(-10));
        assert_eq!(position.entry_price, UsdPrice::new_int(2000));
        assert_eq!(pair_state.short_oi, Quantity::new_int(10));
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn increase_long_blends_entry_price() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 1000));

        let (_state, pair_state, user_state, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(10), // same direction → all opening
            market_order(),
            false,
        )
        .unwrap();

        let position = user_state.positions.get(&eth::DENOM).unwrap();
        assert_eq!(position.size, Quantity::new_int(20));
        // Blended: (10 * $1000 + 10 * $2000) / 20 = $1500
        assert_eq!(position.entry_price, UsdPrice::new_int(1500));
        assert_eq!(pair_state.long_oi, Quantity::new_int(20));
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn flip_long_to_short() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(5),
                oi_weighted_entry_price: UsdValue::new_int(5_000),
                funding_per_unit: FundingPerUnit::new_int(3),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(5, 1000));

        let (_state, pair_state, user_state, vault_pays_user, _upay, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-15), // close 5 long, open 10 short
            market_order(),
            false,
        )
        .unwrap();

        let position = user_state.positions.get(&eth::DENOM).unwrap();
        assert_eq!(position.size, Quantity::new_int(-10));
        // New position: entry_price = exec ≈ $2000, not blended
        assert_eq!(position.entry_price, UsdPrice::new_int(2000));
        // entry_funding set to current pair_state.funding_per_unit
        assert_eq!(position.entry_funding_per_unit, FundingPerUnit::new_int(3));
        // OI flipped
        assert_eq!(pair_state.long_oi, Quantity::ZERO);
        assert_eq!(pair_state.short_oi, Quantity::new_int(10));
        // Realized profit from closing long
        assert!(vault_pays_user > Uint128::ZERO);
    }

    // ====================== Group 6: Funding settlement ======================

    #[test]
    fn funding_settled_positive_accrued() {
        // Long 10 @ $2000, entry_funding=1, pair funding=3.
        // Accrued = 10 * (3 - 1) = 20 → user owes $20 → PnL = -$20.
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(20_000),
                funding_per_unit: FundingPerUnit::new_int(3),
                oi_weighted_entry_funding: UsdValue::new_int(10), // 10 * 1
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), position_with_funding(10, 2000, 1));

        let (_state, _pair_state, _user_state, vault_pays_user, user_pays_vault, _) =
            _submit_order(
                Addr::mock(1),
                Timestamp::from_seconds(0),
                querier,
                &pair_q,
                &mut oracle_q,
                &test_param(),
                default_state(),
                user_state,
                &eth::DENOM,
                Quantity::new_int(-10), // close fully
                market_order(),
                false,
            )
            .unwrap();

        // Closing PnL = 0 (entry == oracle). Funding PnL = -$20.
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::new(20_000_000));
    }

    #[test]
    fn funding_settled_negative_accrued() {
        // Long 10 @ $2000, entry_funding=5, pair funding=3.
        // Accrued = 10 * (3 - 5) = -20 → vault owes user $20 → PnL = +$20.
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(20_000),
                funding_per_unit: FundingPerUnit::new_int(3),
                oi_weighted_entry_funding: UsdValue::new_int(50), // 10 * 5
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), position_with_funding(10, 2000, 5));

        let (_state, _pair_state, _user_state, vault_pays_user, user_pays_vault, _) =
            _submit_order(
                Addr::mock(1),
                Timestamp::from_seconds(0),
                querier,
                &pair_q,
                &mut oracle_q,
                &test_param(),
                default_state(),
                user_state,
                &eth::DENOM,
                Quantity::new_int(-10),
                market_order(),
                false,
            )
            .unwrap();

        // Closing PnL = 0. Funding PnL = +$20.
        assert_eq!(vault_pays_user, Uint128::new(20_000_000));
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    #[test]
    fn no_funding_on_new_position() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, _pair_state, _user_state, vault_pays_user, user_pays_vault, _) =
            _submit_order(
                Addr::mock(1),
                Timestamp::from_seconds(0),
                querier,
                &pair_q,
                &mut oracle_q,
                &test_param(),
                default_state(),
                UserState::default(),
                &eth::DENOM,
                Quantity::new_int(10),
                market_order(),
                false,
            )
            .unwrap();

        // Pure open: no closing PnL, no funding.
        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
    }

    // ================== Group 7: OI and accumulator updates ==================

    #[test]
    fn oi_updated_close_long() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 1000));

        let (_state, pair_state, _user_state, ..) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-3), // close 3
            market_order(),
            false,
        )
        .unwrap();

        assert_eq!(pair_state.long_oi, Quantity::new_int(7));
    }

    #[test]
    fn oi_updated_open_short() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, pair_state, _user_state, ..) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(-5),
            market_order(),
            false,
        )
        .unwrap();

        assert_eq!(pair_state.short_oi, Quantity::new_int(5));
    }

    #[test]
    fn accumulators_removed_on_close() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 1000));

        let (_state, pair_state, _user_state, ..) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10), // full close
            market_order(),
            false,
        )
        .unwrap();

        assert_eq!(pair_state.oi_weighted_entry_price, UsdValue::ZERO);
        assert_eq!(pair_state.oi_weighted_entry_funding, UsdValue::ZERO);
    }

    #[test]
    fn accumulators_added_on_open() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (_state, pair_state, _user_state, ..) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(10),
            market_order(),
            false,
        )
        .unwrap();

        // oi_weighted_entry_price = 10 * $2000 = $20,000
        assert_eq!(
            pair_state.oi_weighted_entry_price,
            UsdValue::new_int(20_000)
        );
        assert_eq!(pair_state.oi_weighted_entry_funding, UsdValue::ZERO);
    }

    // ====================== Group 8: settle_pnl_and_fee ======================

    #[test]
    fn positive_net_pnl_vault_pays_user() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(10_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 1000));

        let (state, _, _, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        assert!(vault_pays_user > Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
        assert!(state.vault_margin < Uint128::new(VAULT_MARGIN));
    }

    #[test]
    fn negative_net_pnl_user_pays_vault() {
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            1000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(20_000),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 2000));

        let (state, _, _, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert!(user_pays_vault > Uint128::ZERO);
        assert!(state.vault_margin > Uint128::new(VAULT_MARGIN));
    }

    #[test]
    fn pnl_exactly_equals_fee() {
        // fee_rate = 5 %, long 10 @ $950, close at exec ≈ $1000.
        // PnL = 10 * ($1000 - $950) = $500.
        // Fee = 10 * $1000 * 0.05 = $500.  Net = $0.
        let param = Param {
            trading_fee_rate: Dimensionless::new_permille(50),
            max_open_orders: 10,
            ..Default::default()
        };

        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            1000,
            test_pair_param(),
            PairState {
                long_oi: Quantity::new_int(10),
                oi_weighted_entry_price: UsdValue::new_int(9_500),
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            },
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let mut user_state = UserState::default();
        user_state
            .positions
            .insert(eth::DENOM.clone(), make_position(10, 950));

        let (state, _, _, vault_pays_user, user_pays_vault, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &param,
            default_state(),
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        assert_eq!(vault_pays_user, Uint128::ZERO);
        assert_eq!(user_pays_vault, Uint128::ZERO);
        assert_eq!(state.vault_margin, Uint128::new(VAULT_MARGIN));
    }

    // ========================== Group 9: End-to-end ==========================

    #[test]
    fn full_open_close_round_trip() {
        // Step 1: open long 10 at $2000.
        let (mock_q, pair_q, mut oracle_q) = setup_queriers(
            2000,
            test_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
            COLLATERAL,
        );
        let querier = QuerierWrapper::new(&mock_q);

        let (state, pair_state, user_state, vpay1, upay1, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier,
            &pair_q,
            &mut oracle_q,
            &test_param(),
            default_state(),
            UserState::default(),
            &eth::DENOM,
            Quantity::new_int(10),
            market_order(),
            false,
        )
        .unwrap();

        assert_eq!(vpay1, Uint128::ZERO);
        assert_eq!(upay1, Uint128::ZERO);
        assert!(user_state.positions.contains_key(&eth::DENOM));

        // Step 2: close the position with fresh queriers using updated pair_state.
        let pair_q2 = NoCachePairQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => test_pair_param() },
            hash_map! { eth::DENOM.clone() => pair_state },
            None,
        );
        let mut oracle_q2 = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price(),
            eth::DENOM.clone() => eth_price(2000),
        });
        let querier2 = QuerierWrapper::new(&mock_q);

        let (state2, pair_state2, user_state2, vpay2, upay2, _) = _submit_order(
            Addr::mock(1),
            Timestamp::from_seconds(0),
            querier2,
            &pair_q2,
            &mut oracle_q2,
            &test_param(),
            state,
            user_state,
            &eth::DENOM,
            Quantity::new_int(-10),
            market_order(),
            false,
        )
        .unwrap();

        // Positions empty after full round trip.
        assert!(user_state2.positions.is_empty());
        // OI back to zero.
        assert_eq!(pair_state2.long_oi, Quantity::ZERO);
        assert_eq!(pair_state2.short_oi, Quantity::ZERO);
        // Accumulators back to zero.
        assert_eq!(pair_state2.oi_weighted_entry_price, UsdValue::ZERO);
        assert_eq!(pair_state2.oi_weighted_entry_funding, UsdValue::ZERO);
        // No PnL (entry == exec == oracle).
        assert_eq!(vpay2, Uint128::ZERO);
        assert_eq!(upay2, Uint128::ZERO);
        // Vault margin unchanged.
        assert_eq!(state2.vault_margin, Uint128::new(VAULT_MARGIN));
    }
}
