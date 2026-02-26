use {
    crate::{
        ASKS, BIDS, NEXT_ORDER_ID, NoCachePerpQuerier, PAIR_PARAMS, PAIR_STATES, PARAM, STATE,
        USER_STATES,
        core::{
            accrue_funding, check_margin, check_minimum_order_size, check_oi_constraint,
            compute_required_margin, compute_target_price, compute_trading_fee, decompose_fill,
            execute_fill, is_price_constraint_violated,
        },
        execute::{BANK, ORACLE},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue, bank,
        perps::{
            Order, OrderId, OrderKind, PairId, PairParam, PairState, Param, State, UserState,
            settlement_currency,
        },
    },
    grug::{
        Addr, Coins, IsZero, MathResult, Message, MutableCtx, Number, NumberConst,
        Order as IterationOrder, QuerierExt, Response, Storage, Uint128, coins,
    },
    std::collections::{BTreeMap, btree_map::Entry},
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

    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    let collateral_balance = ctx
        .querier
        .query_balance(ctx.sender, settlement_currency::DENOM.clone())?;

    let collateral_value = Quantity::from_base(collateral_balance, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // --------------------------- 2. Business logic ---------------------------

    let (payouts, collections, maker_states, order_mutations, order_to_store) = _submit_order(
        ctx.storage,
        ctx.sender,
        ctx.block.timestamp,
        &param,
        &pair_param,
        &mut pair_state,
        &mut taker_state,
        &pair_id,
        oracle_price,
        collateral_value,
        size,
        kind,
        reduce_only,
        &mut oracle_querier,
        settlement_currency_price,
        &mut state,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    STATE.save(ctx.storage, &state)?;

    PAIR_STATES.save(ctx.storage, &pair_id, &pair_state)?;

    USER_STATES.save(ctx.storage, ctx.sender, &taker_state)?;

    for (addr, maker_state) in &maker_states {
        USER_STATES.save(ctx.storage, *addr, maker_state)?;
    }

    let (taker_book, maker_book) = if size.is_positive() {
        (BIDS, ASKS)
    } else {
        (ASKS, BIDS)
    };

    for (stored_price, order_id, mutation) in order_mutations {
        let order_key = (pair_id.clone(), stored_price, order_id);
        match mutation {
            Some(order) => {
                maker_book.save(ctx.storage, order_key, &order)?;
            },
            None => {
                maker_book.remove(ctx.storage, order_key)?;
            },
        }
    }

    if let Some((stored_price, order_id, order)) = order_to_store {
        NEXT_ORDER_ID.save(ctx.storage, &(order_id + OrderId::ONE))?;
        taker_book.save(ctx.storage, (pair_id, stored_price, order_id), &order)?;
    }

    // ---------------------- 4. Perform token transfers -----------------------

    let mut messages = Vec::with_capacity(payouts.len() + collections.len());

    if !payouts.is_empty() {
        messages.push(Message::batch_transfer(payouts.into_iter().map(
            |(addr, amount)| {
                (
                    addr,
                    coins! { settlement_currency::DENOM.clone() => amount },
                )
            },
        ))?);
    }

    for (user, amount) in collections {
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

    Ok(Response::new().add_messages(messages))
}

/// Semi-pure order submission: reads from storage but does not write.
/// All storage mutations are returned as deferred side-effects.
///
/// Mutates (in-memory only):
///
/// - `pair_state` — funding accrued; `long_oi` / `short_oi` updated.
/// - `taker_state.positions` — opened / closed / flipped per fill.
/// - `taker_state.reserved_margin` / `open_order_count` — updated if a
///   limit order remainder is stored.
/// - `state.insurance_fund` — adjusted by settled PnLs.
///
/// Returns:
///
/// - Per-user payouts in settlement-currency base units: `BTreeMap<Addr, Uint128>`.
/// - Per-user collections in settlement-currency base units: `Vec<(Addr, Uint128)>`.
/// - Maker `UserState`s to persist: `BTreeMap<Addr, UserState>`.
/// - Order mutations to apply: `Vec<(OrderKey, Option<Order>)>`.
/// - GTC order to store: `Option<(stored_price, order_id, Order)>`.
fn _submit_order(
    storage: &dyn Storage,
    taker: Addr,
    current_time: grug::Timestamp,
    param: &Param,
    pair_param: &PairParam,
    pair_state: &mut PairState,
    taker_state: &mut UserState,
    pair_id: &PairId,
    oracle_price: UsdPrice,
    collateral_value: UsdValue,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
    oracle_querier: &mut OracleQuerier,
    settlement_price: UsdPrice,
    state: &mut State,
) -> anyhow::Result<(
    BTreeMap<Addr, Uint128>,
    Vec<(Addr, Uint128)>,
    BTreeMap<Addr, UserState>,
    Vec<(UsdPrice, OrderId, Option<Order>)>,
    Option<(UsdPrice, OrderId, Order)>,
)> {
    // ------------- Step 1. Accrue funding before any OI changes --------------

    accrue_funding(pair_state, pair_param, current_time, oracle_price)?;

    // -------------- Step 2. Check minimum order size -------------------------

    if !reduce_only {
        check_minimum_order_size(size, oracle_price, pair_param)?;
    }

    // ----------------------- Step 3. Decompose order -------------------------

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

    // -------------- Step 4. Check OI constraint for opening ------------------

    check_oi_constraint(opening_size, pair_state, pair_param)?;

    // ----------------- Step 5: Pre-match taker margin check ------------------
    //
    // Reduce-only orders only reduce exposure, so they skip the check.

    if !reduce_only {
        let perp_querier = NoCachePerpQuerier::new_local(storage);

        check_margin(
            &perp_querier,
            oracle_querier,
            taker_state,
            param,
            pair_param,
            pair_id,
            oracle_price,
            collateral_value,
            size,
            kind,
        )?;
    }

    // --------------------- Step 6. Compute target price ----------------------

    let taker_is_bid = size.is_positive();
    let target_price = compute_target_price(kind, oracle_price, taker_is_bid)?;

    // ---------------------- Step 7. Match against book -----------------------

    let (unfilled, pnls, maker_states, order_mutations) = match_order(
        storage,
        param,
        pair_id,
        pair_state,
        taker,
        taker_state,
        taker_is_bid,
        target_price,
        fillable_size,
    )?;

    // ------------------- Step 8. Handle unfilled remainder -------------------

    if unfilled.is_non_zero() {
        match kind {
            OrderKind::Market { .. } => {
                ensure!(
                    unfilled < fillable_size,
                    "no liquidity at acceptable price! target_price: {target_price}"
                );
            },
            OrderKind::Limit { limit_price } => {
                let order_to_store = store_limit_order(
                    storage,
                    taker,
                    param,
                    pair_param,
                    taker_state,
                    unfilled,
                    limit_price,
                    reduce_only,
                )?;

                let (payouts, collections) = settle_pnls(pnls, settlement_price, state)?;
                return Ok((
                    payouts,
                    collections,
                    maker_states,
                    order_mutations,
                    Some(order_to_store),
                ));
            },
        }
    }

    let (payouts, collections) = settle_pnls(pnls, settlement_price, state)?;

    Ok((payouts, collections, maker_states, order_mutations, None))
}

/// Mutates:
///
/// - `pair_state.long_oi` / `pair_state.short_oi` — updated per fill.
/// - `taker_state.positions` — opened / closed / flipped per fill.
///
/// Returns:
///
/// - Remaining (unfilled) size (same sign convention as taker's order).
/// - Per-user net PnL in USD (`BTreeMap<Addr, UsdValue>`).
/// - Maker `UserState`s to persist (`BTreeMap<Addr, UserState>`).
/// - Order mutations to apply (`Vec<(OrderKey, Option<Order>)>`):
///   `None` = remove (fully filled), `Some` = update (partially filled).
pub(crate) fn match_order(
    storage: &dyn Storage,
    param: &Param,
    pair_id: &PairId,
    pair_state: &mut PairState,
    taker: Addr,
    taker_state: &mut UserState,
    taker_is_bid: bool,
    target_price: UsdPrice,
    mut remaining_size: Quantity,
) -> anyhow::Result<(
    Quantity,
    BTreeMap<Addr, UsdValue>,
    BTreeMap<Addr, UserState>,
    Vec<(UsdPrice, OrderId, Option<Order>)>,
)> {
    let mut pnls = BTreeMap::new();
    let mut maker_states = BTreeMap::new();
    let mut order_mutations = Vec::new();

    // Create iterator over the maker side of the order book.
    // The iteration follows price-time priority.
    let maker_book = if taker_is_bid {
        ASKS
    } else {
        BIDS
    };

    let maker_orders =
        maker_book
            .prefix(pair_id.clone())
            .range(storage, None, None, IterationOrder::Ascending);

    for record in maker_orders {
        let ((stored_price, maker_order_id), mut maker_order) = record?;

        // If the maker is bid (i.e. taker is ask), we need to "un-invert" the price.
        let resting_price = may_invert_price(stored_price, !taker_is_bid)?;

        // ----------------------- Termination condition -----------------------

        if remaining_size.is_zero() {
            break;
        }

        if is_price_constraint_violated(resting_price, target_price, taker_is_bid) {
            break;
        }

        // ---------------------- Determine fillable size ----------------------

        let opposite = maker_order.size.checked_neg()?;

        let taker_fill_size = if taker_is_bid {
            remaining_size.min(opposite)
        } else {
            remaining_size.max(opposite)
        };

        let maker_fill_size = taker_fill_size.checked_neg()?;

        // -------------------- Settle PnL and trading fee ---------------------

        // Find the maker's user state.
        let maker_state = match maker_states.entry(maker_order.user) {
            Entry::Vacant(e) => {
                let maybe_maker_state = USER_STATES.may_load(storage, maker_order.user)?;
                e.insert(maybe_maker_state.unwrap_or_default())
            },
            Entry::Occupied(e) => e.into_mut(),
        };

        settle_fill(
            pair_id,
            pair_state,
            taker_state,
            taker_fill_size,
            resting_price,
            param.taker_fee_rate,
            &mut pnls,
            taker,
        )?;

        settle_fill(
            pair_id,
            pair_state,
            maker_state,
            maker_fill_size,
            resting_price,
            param.maker_fee_rate,
            &mut pnls,
            maker_order.user,
        )?;

        // ---------------- Update maker's order and user state ----------------

        // Release reserved margin proportionally to the filled portion.
        let margin_to_release = (maker_order.reserved_margin)
            .checked_mul(maker_fill_size)?
            .checked_div(maker_order.size)?;

        maker_state
            .reserved_margin
            .checked_sub_assign(margin_to_release)?;

        maker_order
            .reserved_margin
            .checked_sub_assign(margin_to_release)?;

        maker_order.size.checked_sub_assign(maker_fill_size)?;

        if maker_order.size.is_zero() {
            maker_state.open_order_count -= 1;
            order_mutations.push((stored_price, maker_order_id, None));
        } else {
            order_mutations.push((stored_price, maker_order_id, Some(maker_order)));
        }

        remaining_size.checked_sub_assign(taker_fill_size)?;
    }

    Ok((remaining_size, pnls, maker_states, order_mutations))
}

/// Mutates:
///
/// - `pair_state.long_oi` / `pair_state.short_oi` — updated by `execute_fill`.
/// - `user_state.positions` — opened / closed / flipped by `execute_fill`.
/// - `pnls` — net PnL (pnl − fee) added for `user`.
pub(crate) fn settle_fill(
    pair_id: &PairId,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    fill_size: Quantity,
    fill_price: UsdPrice,
    fee_rate: Dimensionless,
    pnls: &mut BTreeMap<Addr, UsdValue>,
    user: Addr,
) -> grug::MathResult<()> {
    let current_pos = user_state
        .positions
        .get(pair_id)
        .map(|p| p.size)
        .unwrap_or_default();

    let (closing, opening) = decompose_fill(fill_size, current_pos);

    let pnl = execute_fill(
        pair_state, user_state, pair_id, fill_price, closing, opening,
    )?;

    let fee = compute_trading_fee(fill_size, fill_price, fee_rate)?;
    let net = pnl.checked_sub(fee)?;

    pnls.entry(user).or_default().checked_add_assign(net)
}

/// Convert per-user USD PnLs into settlement-currency base-unit amounts and
/// update the insurance fund accordingly.
///
/// Returns:
///
/// - Payouts: users the contract must pay (positive PnL, floor-rounded).
/// - Collections: users who owe the contract (negative PnL, ceil-rounded).
pub(crate) fn settle_pnls(
    pnls: BTreeMap<Addr, UsdValue>,
    settlement_price: UsdPrice,
    state: &mut State,
) -> anyhow::Result<(BTreeMap<Addr, Uint128>, Vec<(Addr, Uint128)>)> {
    let mut payouts = BTreeMap::new();
    let mut collections = Vec::new();

    for (user, net_usd) in pnls {
        if net_usd.is_zero() {
            continue;
        }

        let net_quantity = net_usd.checked_div(settlement_price)?;

        if net_usd > UsdValue::ZERO {
            // Contract pays user: floor rounding favors contract.
            let amount = net_quantity.into_base_floor(settlement_currency::DECIMAL)?;
            if amount.is_non_zero() {
                state.insurance_fund.checked_sub_assign(amount)?;
                payouts.insert(user, amount);
            }
        } else {
            // User pays contract: ceil rounding favors contract.
            let amount = net_quantity
                .checked_abs()?
                .into_base_ceil(settlement_currency::DECIMAL)?;
            if amount.is_non_zero() {
                state.insurance_fund.checked_add_assign(amount)?;
                collections.push((user, amount));
            }
        }
    }

    Ok((payouts, collections))
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

    // Enforce tick size: `limit_price` must be an integer multiple of `tick_size`.
    if pair_param.tick_size.is_non_zero() {
        ensure!(
            limit_price.checked_rem(pair_param.tick_size)?.is_zero(),
            "limit price ({}) is not a multiple of tick size ({})",
            limit_price,
            pair_param.tick_size,
        );
    }

    // Reserve margin for worst case (entire order is opening).
    // Use taker fee rate as worst-case fee reservation.
    let margin_to_reserve = compute_required_margin(size, limit_price, pair_param)?.checked_add(
        compute_trading_fee(size, limit_price, param.taker_fee_rate)?,
    )?;

    user_state.open_order_count += 1;
    (user_state.reserved_margin).checked_add_assign(margin_to_reserve)?;

    // Invert price for buy orders so storage order matches price-time priority.
    let stored_price = may_invert_price(limit_price, size.is_positive())?;

    // Allocate order ID.
    let order_id = NEXT_ORDER_ID.may_load(storage)?.unwrap_or(OrderId::ONE);

    Ok((stored_price, order_id, Order {
        user,
        size,
        reduce_only,
        reserved_margin: margin_to_reserve,
    }))
}

/// When storing a bid order, we "invert" the price such that orders are sorted
/// according to price-time priority. Conversely, when reading orders from the
/// book, we need to "un-invert" the price. This function does both.
fn may_invert_price(price: UsdPrice, is_bid: bool) -> MathResult<UsdPrice> {
    if is_bid {
        UsdPrice::MAX.checked_sub(price)
    } else {
        Ok(price)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::USER_STATES,
        dango_types::{Dimensionless, FundingPerUnit, oracle::PrecisionedPrice, perps::Position},
        grug::{Coins, MockContext, Timestamp, Udec128, Uint64, hash_map},
    };

    const TAKER: Addr = Addr::mock(1);
    const MAKER_A: Addr = Addr::mock(2);
    const MAKER_B: Addr = Addr::mock(3);

    /// Large collateral value that trivially satisfies any margin check.
    const LARGE_COLLATERAL: UsdValue = UsdValue::new_int(999_999_999);

    const SETTLEMENT_PRICE: UsdPrice = UsdPrice::new_int(1);

    fn test_oracle_querier() -> OracleQuerier<'static> {
        OracleQuerier::new_mock(hash_map! {
            pair_id() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100), // $1
                Timestamp::from_seconds(0),
                6,
            ),
        })
    }

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
        let key = (pair_id(), UsdPrice::new_int(price), Uint64::new(order_id));
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
        let key = (pair_id(), inverted_price, Uint64::new(order_id));
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, _, order_mutations, order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100), // 10%
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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

        // Ask should be removed from book (1 removal mutation).
        assert_eq!(order_mutations.len(), 1);
        assert!(order_mutations[0].2.is_none());
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (.., order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (.., order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (.., order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (.., order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (.., order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            true,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10),
            },
            true,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, _, order_mutations, order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // Taker gets a short position.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        assert!(order_to_store.is_none());
        assert_eq!(pair_state.short_oi, Quantity::new_int(10));

        // Bid removed from book (1 removal mutation).
        assert_eq!(order_mutations.len(), 1);
        assert!(order_mutations[0].2.is_none());
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, maker_states, ..) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // Taker: long 10 @ 50000
        let taker_pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(taker_pos.size, Quantity::new_int(10));

        // Maker: short 10 @ 50000
        let maker_pos = maker_states[&MAKER_A].positions.get(&pair_id()).unwrap();
        assert_eq!(maker_pos.size, Quantity::new_int(-10));
        assert_eq!(maker_pos.entry_price, UsdPrice::new_int(50_000));
    }

    // ============= Fee accounting: net PnLs include fees =====================

    #[test]
    fn pnls_include_fees() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (payouts, collections, ..) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // Taker: no realized PnL (opening), fee = |10| * 50000 * 0.001 = 500 USD.
        // Net = -$500 → collection of 500 × 10^6 base units.
        assert!(payouts.is_empty());
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].0, TAKER);
        assert_eq!(collections[0].1, Uint128::new(500_000_000));
    }

    // ======== Tick size enforcement for limit orders =========================

    #[test]
    fn tick_size_valid_multiple_accepted() {
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        // 50,100 is a valid multiple of tick size 100 — should succeed.
        let result = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        );

        assert!(result.is_ok());
    }

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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_050),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, _, order_mutations, order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_100),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));

        // Weighted avg entry: (5*50000 + 5*50100) / 10 = 50050
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_050));

        assert!(order_to_store.is_none());

        // Both asks fully filled (2 removal mutations).
        assert_eq!(order_mutations.len(), 2);
        assert!(order_mutations[0].2.is_none());
        assert!(order_mutations[1].2.is_none());
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, maker_states, ..) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        assert_eq!(maker_states[&MAKER_A].reserved_margin, UsdValue::ZERO);
        assert_eq!(maker_states[&MAKER_A].open_order_count, 0);
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, maker_states, ..) = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(4),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        assert_eq!(maker_states[&MAKER_A].open_order_count, 1);

        // initial_margin = 10 * 50000 / 20 = 25000 USD
        // 40% released, 60% remaining = 15000
        assert_eq!(
            maker_states[&MAKER_A].reserved_margin,
            UsdValue::new_int(15_000)
        );
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
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            Timestamp::from_nanos(0),
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10), // 1%
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("no liquidity at acceptable price")
        );
    }

    // =================== settle_pnls unit tests ==============================

    #[test]
    fn settle_pnls_mixed() {
        let mut state = State {
            insurance_fund: Uint128::new(1_000_000_000),
            ..Default::default()
        };

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(100)),
            (Addr::mock(2), UsdValue::new_int(-200)),
            (Addr::mock(3), UsdValue::ZERO),
        ]);

        let (payouts, collections) = settle_pnls(pnls, SETTLEMENT_PRICE, &mut state).unwrap();

        // Positive PnL: user 1 receives 100 × 10^6 base units.
        assert_eq!(payouts.len(), 1);
        assert_eq!(payouts[&Addr::mock(1)], Uint128::new(100_000_000));

        // Negative PnL: user 2 owes 200 × 10^6 base units.
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0], (Addr::mock(2), Uint128::new(200_000_000)));

        // 1_000_000_000 - 100_000_000 + 200_000_000
        assert_eq!(state.insurance_fund, Uint128::new(1_100_000_000));
    }

    #[test]
    fn settle_pnls_all_payouts() {
        let mut state = State {
            insurance_fund: Uint128::new(1_000_000_000),
            ..Default::default()
        };

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(100)),
            (Addr::mock(2), UsdValue::new_int(50)),
        ]);

        let (payouts, collections) = settle_pnls(pnls, SETTLEMENT_PRICE, &mut state).unwrap();

        assert_eq!(payouts.len(), 2);
        assert!(collections.is_empty());

        // 1_000_000_000 - 100_000_000 - 50_000_000
        assert_eq!(state.insurance_fund, Uint128::new(850_000_000));
    }

    #[test]
    fn settle_pnls_all_collections() {
        let mut state = State::default();

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(-100)),
            (Addr::mock(2), UsdValue::new_int(-200)),
        ]);

        let (payouts, collections) = settle_pnls(pnls, SETTLEMENT_PRICE, &mut state).unwrap();

        assert!(payouts.is_empty());
        assert_eq!(collections.len(), 2);

        // 0 + 100_000_000 + 200_000_000
        assert_eq!(state.insurance_fund, Uint128::new(300_000_000));
    }

    #[test]
    fn settle_pnls_empty() {
        let mut state = State {
            insurance_fund: Uint128::new(500_000_000),
            ..Default::default()
        };

        let pnls = BTreeMap::new();

        let (payouts, collections) = settle_pnls(pnls, SETTLEMENT_PRICE, &mut state).unwrap();

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert_eq!(state.insurance_fund, Uint128::new(500_000_000));
    }
}
