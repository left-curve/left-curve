use {
    crate::{
        ASKS, BIDS, NEXT_ORDER_ID, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{
            accrue_funding, check_minimum_opening, check_oi_constraint, compute_required_margin,
            compute_target_price, compute_trading_fee, decompose_fill, execute_fill,
            is_price_constraint_violated,
        },
        execute::{BANK, ORACLE},
        state::OrderKey,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue, bank,
        perps::{
            Order, OrderId, OrderKind, PairId, PairParam, PairState, Param, UserState,
            settlement_currency,
        },
    },
    grug::{
        Addr, Coins, IsZero, Message, MutableCtx, Number, NumberConst, Order as IterationOrder,
        Response, StdResult, Storage, coins,
    },
    std::{cmp::Ordering, collections::BTreeMap},
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
    let mut state = STATE.load(ctx.storage)?;

    let pair_param = PAIR_PARAMS.load(ctx.storage, &pair_id)?;
    let mut pair_state = PAIR_STATES.load(ctx.storage, &pair_id)?;

    let mut taker_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    let oracle_price = oracle_querier.query_price_for_perps(&pair_id)?;
    let settlement_price = oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // --------------------------- 2. Business logic ---------------------------

    let (transfers, order_to_store) = _submit_order(
        ctx.storage,
        ctx.sender,
        ctx.block.timestamp,
        &param,
        &pair_param,
        &mut pair_state,
        &mut taker_state,
        &pair_id,
        oracle_price,
        size,
        kind,
        reduce_only,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    PAIR_STATES.save(ctx.storage, &pair_id, &pair_state)?;

    USER_STATES.save(ctx.storage, ctx.sender, &taker_state)?;

    if let Some((limit_price, order_id, order)) = order_to_store {
        let next_order_id = order_id + OrderId::ONE;
        let order_key = (pair_id, limit_price, order_id);

        NEXT_ORDER_ID.save(ctx.storage, &next_order_id)?;

        if size.is_positive() {
            BIDS.save(ctx.storage, order_key, &order)?;
        } else {
            ASKS.save(ctx.storage, order_key, &order)?;
        }
    }

    // Convert each user's net USD PnL to settlement currency base units
    // and update the insurance fund. One rounding operation per user.
    let mut messages = Vec::with_capacity(transfers.len());

    for (user, net_usd) in transfers {
        let net_quantity = net_usd.checked_div(settlement_price)?;

        match net_usd.cmp(&UsdValue::ZERO) {
            Ordering::Greater => {
                // Contract pays user: floor rounding favors contract.
                let amount = net_quantity.into_base_floor(settlement_currency::DECIMAL)?;

                if amount.is_non_zero() {
                    state.insurance_fund = state.insurance_fund.checked_sub(amount)?;
                    messages.push(Message::transfer(
                        user,
                        coins! { settlement_currency::DENOM.clone() => amount },
                    )?);
                }
            },
            Ordering::Less => {
                // User pays contract: ceil rounding favors contract.
                let amount = net_quantity
                    .checked_abs()?
                    .into_base_ceil(settlement_currency::DECIMAL)?;

                if amount.is_non_zero() {
                    state.insurance_fund = state.insurance_fund.checked_add(amount)?;
                    messages.push(Message::execute(
                        BANK,
                        &bank::ExecuteMsg::ForceTransfer {
                            from: user,
                            to: ctx.contract,
                            coins: coins! { settlement_currency::DENOM.clone() => amount },
                        },
                        Coins::new(),
                    )?);
                }
            },
            Ordering::Equal => {},
        }
    }

    STATE.save(ctx.storage, &state)?;

    Ok(Response::new().add_messages(messages))
}

/// Mutates:
///
/// - `storage` — resting orders updated/removed during matching; maker
///   `UserState`s saved after each fill.
/// - `pair_state` — funding accrued; `long_oi` / `short_oi` updated.
/// - `taker_state.positions` — opened / closed / flipped per fill.
/// - `taker_state.reserved_margin` / `open_order_count` — updated if a
///   limit order remainder is stored.
///
/// Returns:
///
/// - Per-user net PnL in USD: `BTreeMap<Addr, UsdValue>`.
/// - GTC order to store: `Option<(stored_price, order_id, Order)>`.
fn _submit_order(
    storage: &mut dyn Storage,
    sender: Addr,
    current_time: grug::Timestamp,
    param: &Param,
    pair_param: &PairParam,
    pair_state: &mut PairState,
    taker_state: &mut UserState,
    pair_id: &PairId,
    oracle_price: UsdPrice,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
) -> anyhow::Result<(BTreeMap<Addr, UsdValue>, Option<(UsdPrice, OrderId, Order)>)> {
    // ------------- Step 1. Accrue funding before any OI changes --------------

    accrue_funding(pair_state, pair_param, current_time, oracle_price)?;

    // ----------------------- Step 2. Decompose order -------------------------

    let current_position = taker_state
        .positions
        .get(pair_id)
        .map(|p| p.size)
        .unwrap_or_default();

    let (closing_size, mut opening_size) = decompose_fill(size, current_position);

    if reduce_only {
        opening_size = Quantity::ZERO;
    }

    let fillable_size = closing_size.checked_add(opening_size)?;

    ensure!(fillable_size.is_non_zero(), "fillable size is zero");

    // -------------- Step 3. Check minimum opening constraint -----------------

    check_minimum_opening(opening_size, oracle_price, pair_param)?;

    // -------------- Step 4. Check OI constraint for opening ------------------

    check_oi_constraint(opening_size, pair_state, pair_param)?;

    // ------------------- Step 5. Compute target price ------------------------

    let is_bid = size.is_positive();
    let target_price = compute_target_price(kind, oracle_price, is_bid)?;

    // ---------------------- Step 6. Match against book ------------------------

    let (filled_size, transfers) = match_order(
        storage,
        sender,
        param,
        pair_param,
        pair_state,
        taker_state,
        pair_id,
        fillable_size,
        target_price,
        is_bid,
    )?;

    let unfilled = fillable_size.checked_sub(filled_size)?;

    // ------------- Step 7. Handle unfilled remainder -------------------------

    if unfilled.is_non_zero() {
        match kind {
            OrderKind::Market { .. } => {
                // IOC: cancel remainder. Error if nothing was filled at all.
                ensure!(
                    filled_size.is_non_zero(),
                    "no liquidity at acceptable price! target_price: {}",
                    target_price
                );
            },
            OrderKind::Limit { limit_price } => {
                // GTC: store remainder as a resting limit order.
                let order_to_store = store_limit_order(
                    storage,
                    sender,
                    param,
                    pair_param,
                    taker_state,
                    unfilled,
                    limit_price,
                    reduce_only,
                )?;

                return Ok((transfers, Some(order_to_store)));
            },
        }
    }

    Ok((transfers, None))
}

/// Walk the opposite side of the book, filling at each resting order's price
/// until the taker order is exhausted or no more acceptable prices exist.
///
/// Mutates:
///
/// - `storage` — resting orders updated/removed; maker `UserState`s saved.
/// - `pair_state.long_oi` / `pair_state.short_oi` — updated per fill.
/// - `taker_state.positions` — opened / closed / flipped per fill.
///
/// Returns:
///
/// - Total size filled (same sign convention as taker's order).
/// - Per-user net PnL in USD: `BTreeMap<Addr, UsdValue>`.
///   Positive = user gains, negative = user loses.
///   Includes both fill PnL (funding + realized) and trading fees
///   (subtracted).
fn match_order(
    storage: &mut dyn Storage,
    sender: Addr,
    param: &Param,
    _pair_param: &PairParam,
    pair_state: &mut PairState,
    taker_state: &mut UserState,
    pair_id: &PairId,
    mut remaining_size: Quantity,
    target_price: UsdPrice,
    is_bid: bool,
) -> anyhow::Result<(Quantity, BTreeMap<Addr, UsdValue>)> {
    let mut transfers: BTreeMap<Addr, UsdValue> = BTreeMap::new();
    let total_size = remaining_size;

    // Collect resting orders from the opposite side of the book.
    // For a buy (bid), we match against asks (ascending by price = best ask first).
    // For a sell (ask), we match against bids (ascending by inverted price = best bid first).
    // prefix() strips the PairId, so results are (UsdPrice, OrderId) suffixes.
    let resting_orders: Vec<((UsdPrice, OrderId), Order)> = if is_bid {
        ASKS.prefix(pair_id.clone())
            .range(storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?
    } else {
        BIDS.prefix(pair_id.clone())
            .range(storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?
    };

    for ((stored_price, order_id), resting_order) in resting_orders {
        if remaining_size.is_zero() {
            break;
        }

        // Recover the real price: bids are stored inverted.
        let resting_price = if is_bid {
            stored_price // asks stored in natural order
        } else {
            UsdPrice::MAX.checked_sub(stored_price)? // un-invert bid price
        };

        // Price check: stop if the resting price is worse than the taker's target.
        if is_price_constraint_violated(resting_price, target_price, is_bid) {
            break;
        }

        // Determine fill size from taker's perspective (same sign as taker's order).
        // Resting order has opposite sign, so negate it to get taker's sign convention,
        // then clamp to the smaller magnitude.
        let opposite = resting_order.size.checked_neg()?;
        let taker_fill_size = if is_bid {
            remaining_size.min(opposite)
        } else {
            remaining_size.max(opposite)
        };

        let fill_abs = taker_fill_size.checked_abs()?;
        let resting_abs = resting_order.size.checked_abs()?;

        // --- Taker side ---
        let taker_current_pos = taker_state
            .positions
            .get(pair_id)
            .map(|p| p.size)
            .unwrap_or_default();
        let (taker_closing, taker_opening) = decompose_fill(taker_fill_size, taker_current_pos);

        let taker_pnl = execute_fill(
            pair_state,
            taker_state,
            pair_id,
            resting_price,
            taker_closing,
            taker_opening,
        )?;

        let taker_fee = compute_trading_fee(taker_fill_size, resting_price, param.taker_fee_rate)?;
        let taker_net = taker_pnl.checked_sub(taker_fee)?;

        transfers
            .entry(sender)
            .or_default()
            .checked_add_assign(taker_net)?;

        // --- Maker side ---
        let maker_addr = resting_order.user;

        let mut maker_state = USER_STATES
            .may_load(storage, maker_addr)?
            .unwrap_or_default();

        // Maker fill size is opposite sign from taker.
        let maker_fill_size = taker_fill_size.checked_neg()?;

        let maker_current_pos = maker_state
            .positions
            .get(pair_id)
            .map(|p| p.size)
            .unwrap_or_default();
        let (maker_closing, maker_opening) = decompose_fill(maker_fill_size, maker_current_pos);

        let maker_pnl = execute_fill(
            pair_state,
            &mut maker_state,
            pair_id,
            resting_price,
            maker_closing,
            maker_opening,
        )?;

        let maker_fee = compute_trading_fee(maker_fill_size, resting_price, param.maker_fee_rate)?;

        // Release reserved margin proportionally to the filled portion.
        let proportion = fill_abs.checked_div(resting_abs)?;
        let margin_to_release = resting_order.reserved_margin.checked_mul(proportion)?;
        maker_state
            .reserved_margin
            .checked_sub_assign(margin_to_release)?;

        // Update or remove the resting order.
        let order_key: OrderKey = (pair_id.clone(), stored_price, order_id);
        let resting_remaining = resting_abs.checked_sub(fill_abs)?;

        if resting_remaining.is_zero() {
            // Fully filled: remove order.
            if is_bid {
                ASKS.remove(storage, order_key)?;
            } else {
                BIDS.remove(storage, order_key)?;
            }
            maker_state.open_order_count -= 1;
        } else {
            // Partially filled: update size and reserved margin.
            let new_size = if is_bid {
                resting_remaining.checked_neg()? // ask has negative size
            } else {
                resting_remaining // bid has positive size
            };
            let new_reserved = resting_order
                .reserved_margin
                .checked_sub(margin_to_release)?;

            let updated_order = Order {
                user: maker_addr,
                size: new_size,
                reduce_only: resting_order.reduce_only,
                reserved_margin: new_reserved,
            };

            if is_bid {
                ASKS.save(storage, order_key, &updated_order)?;
            } else {
                BIDS.save(storage, order_key, &updated_order)?;
            }
        }

        // Save maker state.
        USER_STATES.save(storage, maker_addr, &maker_state)?;

        // Accumulate maker transfer.
        let maker_net = maker_pnl.checked_sub(maker_fee)?;
        transfers
            .entry(maker_addr)
            .or_default()
            .checked_add_assign(maker_net)?;

        // Reduce remaining size.
        remaining_size = remaining_size.checked_sub(taker_fill_size)?;
    }

    let filled_size = total_size.checked_sub(remaining_size)?;

    Ok((filled_size, transfers))
}

fn store_limit_order(
    storage: &dyn Storage,
    user: Addr,
    param: &Param,
    pair_param: &PairParam,
    user_state: &mut UserState,
    size: Quantity,
    limit_price: UsdPrice,
    reduce_only: bool,
) -> anyhow::Result<(UsdPrice, OrderId, Order)> {
    ensure!(
        user_state.open_order_count < param.max_open_orders,
        "too many open orders! max allowed: {}",
        param.max_open_orders
    );

    // Enforce tick size: limit_price must be an integer multiple of tick_size.
    // Divide, floor to integer, multiply back, and verify equality.
    if pair_param.tick_size.is_non_zero() {
        let ratio = limit_price.checked_div(pair_param.tick_size)?;
        let floored_int = ratio.into_inner().into_int_floor();
        let reconstructed =
            Dimensionless::new_int(floored_int.0).checked_mul(pair_param.tick_size)?;
        ensure!(
            reconstructed == limit_price,
            "limit price {} is not a multiple of tick size {}",
            limit_price,
            pair_param.tick_size,
        );
    }

    // Reserve margin for worst case (entire order is opening).
    // Use taker fee rate as worst-case fee reservation.
    let margin_to_reserve = compute_required_margin(size, limit_price, pair_param)?.checked_add(
        size.checked_abs()?
            .checked_mul(limit_price)?
            .checked_mul(param.taker_fee_rate)?,
    )?;

    user_state.open_order_count += 1;
    user_state
        .reserved_margin
        .checked_add_assign(margin_to_reserve)?;

    // Invert price for buy orders so storage order matches price-time priority.
    let stored_price = if size.is_positive() {
        UsdPrice::MAX.checked_sub(limit_price)?
    } else {
        limit_price
    };

    // Allocate order ID.
    let order_id = NEXT_ORDER_ID.may_load(storage)?.unwrap_or(OrderId::ONE);

    Ok((stored_price, order_id, Order {
        user,
        size,
        reduce_only,
        reserved_margin: margin_to_reserve,
    }))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::USER_STATES,
        dango_types::{Dimensionless, FundingPerUnit, perps::Position},
        grug::{Coins, MockContext, Timestamp, Uint64},
    };

    const TAKER: Addr = Addr::mock(1);
    const MAKER_A: Addr = Addr::mock(2);
    const MAKER_B: Addr = Addr::mock(3);

    fn pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn test_param() -> Param {
        Param {
            max_open_orders: 10,
            taker_fee_rate: Dimensionless::new_permille(1), // 0.1%
            maker_fee_rate: Dimensionless::ZERO,
            ..Default::default()
        }
    }

    fn test_pair_param() -> PairParam {
        PairParam {
            skew_scale: Quantity::new_int(1_000),
            max_abs_oi: Quantity::new_int(1_000_000),
            tick_size: UsdPrice::new_int(1),
            initial_margin_ratio: Dimensionless::new_permille(50), // 5%
            maintenance_margin_ratio: Dimensionless::new_permille(25), // 2.5%
            ..Default::default()
        }
    }

    fn setup_storage(storage: &mut dyn Storage) {
        PARAM.save(storage, &test_param()).unwrap();
        PAIR_PARAMS
            .save(storage, &pair_id(), &test_pair_param())
            .unwrap();
        PAIR_STATES
            .save(
                storage,
                &pair_id(),
                &PairState::new(Timestamp::from_nanos(0)),
            )
            .unwrap();
        NEXT_ORDER_ID.save(storage, &Uint64::new(1)).unwrap();
    }

    /// Place a resting ask (sell) order on the book.
    fn place_ask(storage: &mut dyn Storage, maker: Addr, price: i128, size: i128, order_id: u64) {
        let key: OrderKey = (pair_id(), UsdPrice::new_int(price), Uint64::new(order_id));
        let order = Order {
            user: maker,
            size: Quantity::new_int(-size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(size.abs() * price / 20), // 5% margin
        };
        ASKS.save(storage, key, &order).unwrap();

        let mut maker_state = USER_STATES
            .may_load(storage, maker)
            .unwrap()
            .unwrap_or_default();
        maker_state.open_order_count += 1;
        maker_state
            .reserved_margin
            .checked_add_assign(order.reserved_margin)
            .unwrap();
        USER_STATES.save(storage, maker, &maker_state).unwrap();
    }

    /// Place a resting bid (buy) order on the book.
    fn place_bid(storage: &mut dyn Storage, maker: Addr, price: i128, size: i128, order_id: u64) {
        let inverted_price = UsdPrice::MAX.checked_sub(UsdPrice::new_int(price)).unwrap();
        let key: OrderKey = (pair_id(), inverted_price, Uint64::new(order_id));
        let order = Order {
            user: maker,
            size: Quantity::new_int(size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(size.abs() * price / 20),
        };
        BIDS.save(storage, key, &order).unwrap();

        let mut maker_state = USER_STATES
            .may_load(storage, maker)
            .unwrap()
            .unwrap_or_default();
        maker_state.open_order_count += 1;
        maker_state
            .reserved_margin
            .checked_add_assign(order.reserved_margin)
            .unwrap();
        USER_STATES.save(storage, maker, &maker_state).unwrap();
    }

    // =================== Market buy: single fill, full fill ==================

    #[test]
    fn market_buy_single_full_fill() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100), // 10%
            },
            false,
        )
        .unwrap();

        // Taker should have a long position.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        // No order stored (market order, fully filled).
        assert!(order_to_store.is_none());

        // OI updated.
        assert_eq!(pair_state.long_oi, Quantity::new_int(10));

        // Ask should be removed from book.
        let remaining_asks: Vec<_> = ASKS
            .prefix(pair_id())
            .range(&ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert!(remaining_asks.is_empty());
    }

    // ============= Market buy: partial fill (IOC cancels remainder) ===========

    #[test]
    fn market_buy_partial_fill_ioc() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        // Taker gets 5 (partial), remainder canceled (IOC).
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(5));

        assert!(order_to_store.is_none());
    }

    // ============= Market buy: no liquidity at acceptable price ==============

    #[test]
    fn market_buy_no_liquidity_errors() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let err = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10),
            },
            false,
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("no liquidity at acceptable price")
        );
    }

    // ============= Limit buy: fully fills against book =======================

    #[test]
    fn limit_buy_fully_fills() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
            },
            false,
        )
        .unwrap();

        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));

        assert!(order_to_store.is_none());
    }

    // ============ Limit buy: partial fill, remainder stored as GTC ============

    #[test]
    fn limit_buy_partial_fill_remainder_stored() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
            },
            false,
        )
        .unwrap();

        // 5 filled.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(5));

        // Remainder (5) stored as GTC.
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(5));
    }

    // ====== Limit buy: no matchable orders, entire order stored ==============

    #[test]
    fn limit_buy_no_match_stored_entirely() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 51_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
            },
            false,
        )
        .unwrap();

        // No position (nothing filled).
        assert!(!taker_state.positions.contains_key(&pair_id()));

        // Entire order stored.
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(10));
    }

    // ================== Reduce-only: only closes existing ====================

    #[test]
    fn reduce_only_closes_only() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 48_000, 20, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        pair_state.long_oi = Quantity::new_int(5);

        let mut taker_state = UserState::default();
        taker_state.positions.insert(pair_id(), Position {
            size: Quantity::new_int(5),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            true,
        )
        .unwrap();

        // Position fully closed.
        assert!(!taker_state.positions.contains_key(&pair_id()));
        assert!(order_to_store.is_none());
    }

    // ============= Reduce-only with no position errors =======================

    #[test]
    fn reduce_only_no_position_errors() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let err = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10),
            },
            true,
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("fillable size is zero")
        );
    }

    // ============= Market sell against resting bids ==========================

    #[test]
    fn market_sell_against_bids() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        // Taker gets a short position.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        assert!(order_to_store.is_none());
        assert_eq!(pair_state.short_oi, Quantity::new_int(10));

        // Bid removed from book.
        let remaining_bids: Vec<_> = BIDS
            .prefix(pair_id())
            .range(&ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert!(remaining_bids.is_empty());
    }

    // =========== Two-sided settlement: both positions updated =================

    #[test]
    fn two_sided_settlement() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let _ = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        // Taker: long 10 @ 50000
        let taker_pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(taker_pos.size, Quantity::new_int(10));

        // Maker: short 10 @ 50000
        let maker_state = USER_STATES.load(&ctx.storage, MAKER_A).unwrap();
        let maker_pos = maker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(maker_pos.size, Quantity::new_int(-10));
        assert_eq!(maker_pos.entry_price, UsdPrice::new_int(50_000));
    }

    // =========== Fee accounting: net transfers include fees ====================

    #[test]
    fn transfers_include_fees() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (transfers, _) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        // Taker: no realized PnL (opening), fee = |10| * 50000 * 0.001 = 500 USD.
        // Net = 0 - 500 = -500 USD.
        assert_eq!(transfers[&TAKER], UsdValue::new_int(-500));

        // Maker: no realized PnL (opening), fee = 0%.
        // Net = 0.
        assert_eq!(transfers[&MAKER_A], UsdValue::ZERO);
    }

    // ======== Tick size enforcement for limit orders =========================

    #[test]
    fn tick_size_enforcement() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let mut pair_param = test_pair_param();
        pair_param.tick_size = UsdPrice::new_int(100);
        PAIR_PARAMS
            .save(&mut ctx.storage, &pair_id(), &pair_param)
            .unwrap();

        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let err = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_050),
            },
            false,
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("not a multiple of tick size")
        );
    }

    // ======= Multi-level fill: walks through price levels ====================

    #[test]
    fn market_buy_walks_price_levels() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);
        place_ask(&mut ctx.storage, MAKER_B, 50_100, 5, 101);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let (_, order_to_store) = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_100),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));

        // Weighted avg entry: (5*50000 + 5*50100) / 10 = 50050
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_050));

        assert!(order_to_store.is_none());

        let remaining_asks: Vec<_> = ASKS
            .prefix(pair_id())
            .range(&ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert!(remaining_asks.is_empty());
    }

    // ======= Maker reserved margin release: full fill ========================

    #[test]
    fn maker_reserved_margin_released_on_full_fill() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let maker_state_before = USER_STATES.load(&ctx.storage, MAKER_A).unwrap();
        assert!(maker_state_before.reserved_margin.is_non_zero());
        assert_eq!(maker_state_before.open_order_count, 1);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let _ = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        let maker_state_after = USER_STATES.load(&ctx.storage, MAKER_A).unwrap();
        assert_eq!(maker_state_after.reserved_margin, UsdValue::ZERO);
        assert_eq!(maker_state_after.open_order_count, 0);
    }

    // ======= Maker reserved margin release: partial fill =====================

    #[test]
    fn maker_reserved_margin_released_on_partial_fill() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let _ = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(4),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
        )
        .unwrap();

        let maker_state_after = USER_STATES.load(&ctx.storage, MAKER_A).unwrap();
        assert_eq!(maker_state_after.open_order_count, 1);

        // initial_margin = 10 * 50000 / 20 = 25000 USD
        // 40% released, 60% remaining = 15000
        assert_eq!(maker_state_after.reserved_margin, UsdValue::new_int(15_000));
    }

    // ======= Market buy: price beyond slippage ===============================

    #[test]
    fn market_buy_slippage_exceeded() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 60_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();

        let err = _submit_order(
            &mut ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10), // 1%
            },
            false,
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("no liquidity at acceptable price")
        );
    }
}
