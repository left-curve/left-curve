use {
    crate::{
        ASKS, BIDS, NEXT_ORDER_ID, NoCachePerpQuerier, PAIR_PARAMS, PAIR_STATES, PARAM, STATE,
        USER_STATES,
        core::{
            check_margin, check_minimum_order_size, check_oi_constraint, compute_available_margin,
            compute_required_margin, compute_target_price, compute_trading_fee, decompose_fill,
            execute_fill, is_price_constraint_violated,
        },
        execute::{BANK, ORACLE},
        price::may_invert_price,
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
        Addr, Coins, IsZero, Message, MutableCtx, Number, NumberConst, Order as IterationOrder,
        QuerierExt, Response, Storage, Uint128, coins,
    },
    std::{
        cmp::Ordering,
        collections::{BTreeMap, btree_map::Entry},
    },
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
        ctx.contract,
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
/// - `state.vault_margin` — adjusted by settled PnLs.
///
/// Returns:
///
/// - Per-user payouts in settlement-currency base units: `BTreeMap<Addr, Uint128>`.
/// - Per-user collections in settlement-currency base units: `BTreeMap<Addr, Uint128>`.
/// - Maker `UserState`s to persist: `BTreeMap<Addr, UserState>`.
/// - Order mutations to apply: `Vec<(OrderKey, Option<Order>)>`.
/// - GTC order to store: `Option<(stored_price, order_id, Order)>`.
fn _submit_order(
    storage: &dyn Storage,
    taker: Addr,
    contract: Addr,
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
    BTreeMap<Addr, Uint128>,
    BTreeMap<Addr, UserState>,
    Vec<(UsdPrice, OrderId, Option<Order>)>,
    Option<(UsdPrice, OrderId, Order)>,
)> {
    // -------------- Step 1. Check minimum order size -------------------------

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

    // ---------------------- Step 5. Post-only fast path ----------------------

    if let Some(limit_price) = kind.post_only_price() {
        let order_to_store = store_post_only_limit_order(
            storage,
            taker,
            param,
            pair_param,
            taker_state,
            pair_id,
            fillable_size,
            limit_price,
            reduce_only,
            collateral_value,
            oracle_querier,
        )?;

        return Ok((
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            Vec::new(),
            Some(order_to_store),
        ));
    }

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
            pair_id,
            oracle_price,
            collateral_value,
            size,
        )?;
    }

    // --------------------- Step 6. Compute target price ----------------------

    let taker_is_bid = size.is_positive();
    let target_price = compute_target_price(kind, oracle_price, taker_is_bid)?;

    // ---------------------- Step 7. Match against book -----------------------

    let mut maker_states = BTreeMap::new();

    let (unfilled, pnls, fees, order_mutations) = match_order(
        storage,
        param,
        pair_id,
        pair_state,
        taker,
        taker_state,
        taker_is_bid,
        target_price,
        fillable_size,
        &mut maker_states,
    )?;

    // ------------------- Step 8. Handle unfilled remainder -------------------

    let order_to_store = if unfilled.is_non_zero() {
        match kind {
            OrderKind::Limit { limit_price, .. } => Some(store_limit_order(
                storage,
                taker,
                param,
                pair_param,
                taker_state,
                unfilled,
                limit_price,
                reduce_only,
                collateral_value,
                oracle_querier,
            )?),
            OrderKind::Market { .. } => {
                ensure!(
                    unfilled < fillable_size,
                    "no liquidity at acceptable price! target_price: {target_price}"
                );

                None
            },
        }
    } else {
        None
    };

    let (payouts, collections) = settle_pnls(pnls, fees, settlement_price, state, contract)?;

    Ok((
        payouts,
        collections,
        maker_states,
        order_mutations,
        order_to_store,
    ))
}

/// Mutates:
///
/// - `pair_state.long_oi` / `pair_state.short_oi` — updated per fill.
/// - `taker_state.positions` — opened / closed / flipped per fill.
/// - `maker_states` — each matched maker's `UserState` is loaded from storage
///   (if not already present) and updated in place.
///
/// Returns:
///
/// - Remaining (unfilled) size (same sign convention as taker's order).
/// - Per-user position PnL in USD (`BTreeMap<Addr, UsdValue>`).
/// - Per-user trading fees in USD (`BTreeMap<Addr, UsdValue>`).
/// - Order mutations to apply (`Vec<(StoredPrice, OrderId, Option<Order>)>`):
///   `None` = remove (fully filled), `Some` = update (partially filled).
///
/// Self-trade prevention (EXPIRE_MAKER): if a resting order belongs to
/// the taker, the order is cancelled and the taker continues matching
/// deeper in the book.
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
    maker_states: &mut BTreeMap<Addr, UserState>,
) -> anyhow::Result<(
    Quantity,
    BTreeMap<Addr, UsdValue>,
    BTreeMap<Addr, UsdValue>,
    Vec<(UsdPrice, OrderId, Option<Order>)>,
)> {
    let mut pnls = BTreeMap::new();
    let mut fees = BTreeMap::new();
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
        let resting_price = may_invert_price(stored_price, !taker_is_bid);

        // ----------------------- Termination condition -----------------------

        if remaining_size.is_zero() {
            break;
        }

        if is_price_constraint_violated(resting_price, target_price, taker_is_bid) {
            break;
        }

        // --------- Self-trade prevention (EXPIRE_MAKER) ----------

        // If we come across a maker order that was placed by the taker himself,
        // cancel the maker order and move on.
        // This is consistent with industry standard practice. Specifically, it
        // corresponds to Binance's EXPIRE_MAKER mode:
        // https://developers.binance.com/docs/binance-spot-api-docs/faqs/stp_faq
        if maker_order.user == taker {
            taker_state.open_order_count -= 1;
            (taker_state.reserved_margin).checked_sub_assign(maker_order.reserved_margin)?;

            order_mutations.push((stored_price, maker_order_id, None));

            continue;
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
            &mut fees,
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
            &mut fees,
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

    Ok((remaining_size, pnls, fees, order_mutations))
}

/// Mutates:
///
/// - `pair_state.long_oi` / `pair_state.short_oi` — updated by `execute_fill`.
/// - `user_state.positions` — opened / closed / flipped by `execute_fill`.
/// - `pnls` — position PnL added for `user`.
/// - `fees` — trading fee added for `user`.
pub(crate) fn settle_fill(
    pair_id: &PairId,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    fill_size: Quantity,
    fill_price: UsdPrice,
    fee_rate: Dimensionless,
    pnls: &mut BTreeMap<Addr, UsdValue>,
    fees: &mut BTreeMap<Addr, UsdValue>,
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

    pnls.entry(user).or_default().checked_add_assign(pnl)?;
    fees.entry(user).or_default().checked_add_assign(fee)
}

/// Settle PnLs and fees into settlement-currency base-unit amounts.
///
/// Two loops:
/// 1. **Fee loop** (first): non-vault fees increase `vault_margin` and become
///    collections. Vault fees are skipped (paying yourself is a no-op).
/// 2. **PnL loop** (second): vault PnL adjusts `vault_margin` (with bad-debt
///    tracking). Non-vault PnL becomes payouts (profit) or collections (loss)
///    without touching `vault_margin` — the losing counterparty pays the
///    winning one directly.
///
/// Mutates:
///
/// - `state.vault_margin` — adjusted by vault PnL and non-vault fees.
/// - `state.adl_deficit` — increased if vault loss exceeds vault margin.
///
/// Returns:
///
/// - Payouts: users the contract must pay (positive PnL, floor-rounded).
/// - Collections: users who owe the contract (negative PnL + fees, ceil-rounded).
pub(crate) fn settle_pnls(
    pnls: BTreeMap<Addr, UsdValue>,
    fees: BTreeMap<Addr, UsdValue>,
    settlement_price: UsdPrice,
    state: &mut State,
    contract: Addr,
) -> anyhow::Result<(BTreeMap<Addr, Uint128>, BTreeMap<Addr, Uint128>)> {
    let mut payouts: BTreeMap<Addr, Uint128> = BTreeMap::new();
    let mut collections: BTreeMap<Addr, Uint128> = BTreeMap::new();

    // ---- Fee loop (first: collect fees so they help absorb vault losses) ----
    for (user, fee) in fees {
        if fee.is_zero() || user == contract {
            continue;
        }

        // Non-vault fee → vault_margin increases, user pays.
        let amount = fee
            .checked_div(settlement_price)?
            .into_base_ceil(settlement_currency::DECIMAL)?;

        if amount.is_non_zero() {
            state.vault_margin = state.vault_margin.checked_add(amount)?;
            *collections.entry(user).or_default() = collections
                .get(&user)
                .copied()
                .unwrap_or_default()
                .checked_add(amount)?;
        }
    }

    // ---- PnL loop (second: vault losses can absorb from fee-augmented margin) ----
    for (user, pnl) in pnls {
        if pnl.is_zero() {
            continue;
        }

        let quantity = pnl.checked_div(settlement_price)?;

        match (user == contract, pnl.cmp(&UsdValue::ZERO)) {
            // Vault realizes a profit.
            (true, Ordering::Greater) => {
                let amount = quantity.into_base_ceil(settlement_currency::DECIMAL)?;

                if amount.is_non_zero() {
                    // First repay adl_deficit, then increase vault_margin.
                    let repaid = amount.min(state.adl_deficit);
                    let remainder = amount.checked_sub(repaid)?;

                    state.adl_deficit.checked_sub_assign(repaid)?;
                    state.vault_margin = state.vault_margin.checked_add(remainder)?;
                }
            },
            // Vault realizes a loss.
            (true, Ordering::Less) => {
                let amount = quantity
                    .checked_abs()?
                    .into_base_ceil(settlement_currency::DECIMAL)?;

                if amount.is_non_zero() {
                    let absorbed = amount.min(state.vault_margin);
                    let unabsorbed = amount.checked_sub(absorbed)?;

                    state.vault_margin.checked_sub_assign(absorbed)?;
                    state.adl_deficit.checked_add_assign(unabsorbed)?;
                }
            },
            // Non-vault user realizes a profit: payout.
            (false, Ordering::Greater) => {
                let amount = quantity.into_base_floor(settlement_currency::DECIMAL)?;

                if amount.is_non_zero() {
                    payouts
                        .entry(user)
                        .or_default()
                        .checked_add_assign(amount)?;
                }
            },
            // Non-vault user realizes a loss: collection.
            (false, Ordering::Less) => {
                let amount = quantity
                    .checked_abs()?
                    .into_base_ceil(settlement_currency::DECIMAL)?;

                if amount.is_non_zero() {
                    collections
                        .entry(user)
                        .or_default()
                        .checked_add_assign(amount)?;
                }
            },
            // Zero PnL -- nothing to do.
            (_, Ordering::Equal) => {},
        }
    }

    Ok((payouts, collections))
}

/// Validate and store a post-only limit order. Rejects if the limit price
/// would cross the best resting order on the opposite side of the book.
///
/// Mutates:
///
/// - `taker_state.reserved_margin` — increased by the margin reserved for
///   the resting order.
/// - `taker_state.open_order_count` — incremented by one.
///
/// Returns:
///
/// - `(stored_price, order_id, Order)` — the resting order to persist.
fn store_post_only_limit_order(
    storage: &dyn Storage,
    taker: Addr,
    param: &Param,
    pair_param: &PairParam,
    taker_state: &mut UserState,
    pair_id: &PairId,
    size: Quantity,
    limit_price: UsdPrice,
    reduce_only: bool,
    collateral_value: UsdValue,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<(UsdPrice, OrderId, Order)> {
    let taker_is_bid = size.is_positive();
    let maker_is_bid = !taker_is_bid;

    let maker_book = if taker_is_bid {
        ASKS
    } else {
        BIDS
    };

    if let Some(record) = maker_book
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .next()
    {
        let ((stored_price, _), _) = record?;
        let best_price = may_invert_price(stored_price, maker_is_bid);

        if taker_is_bid {
            ensure!(
                limit_price < best_price,
                "post-only buy at {limit_price} would cross best ask at {best_price}"
            );
        } else {
            ensure!(
                limit_price > best_price,
                "post-only sell at {limit_price} would cross best bid at {best_price}"
            );
        }
    }

    store_limit_order(
        storage,
        taker,
        param,
        pair_param,
        taker_state,
        size,
        limit_price,
        reduce_only,
        collateral_value,
        oracle_querier,
    )
}

/// Mutates:
///
/// - `user_state.reserved_margin` — increased by the margin reserved for
///   the resting order.
/// - `user_state.open_order_count` — incremented by one.
///
/// Returns:
///
/// - `(stored_price, order_id, Order)` — the resting order to persist.
fn store_limit_order(
    storage: &dyn Storage,
    user: Addr,
    param: &Param,
    pair_param: &PairParam,
    user_state: &mut UserState,
    size: Quantity,
    limit_price: UsdPrice,
    reduce_only: bool,
    collateral_value: UsdValue,
    oracle_querier: &mut OracleQuerier,
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
    let margin_to_reserve = compute_required_margin(size, limit_price, pair_param)?;

    // 0%-fill margin check: verify the user can afford this reservation.
    if !reduce_only {
        let perp_querier = NoCachePerpQuerier::new_local(storage);

        let available_margin = compute_available_margin(
            collateral_value,
            user_state,
            &perp_querier,
            oracle_querier,
            user_state.reserved_margin,
        )?;

        ensure!(
            available_margin >= margin_to_reserve,
            "insufficient margin for limit order: available ({}) < required ({})",
            available_margin,
            margin_to_reserve
        );
    }

    user_state.open_order_count += 1;
    (user_state.reserved_margin).checked_add_assign(margin_to_reserve)?;

    // Invert price for buy orders so storage order matches price-time priority.
    let stored_price = may_invert_price(limit_price, size.is_positive());

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
        dango_types::{Dimensionless, FundingPerUnit, oracle::PrecisionedPrice, perps::Position},
        grug::{Coins, MockContext, Timestamp, Udec128, Uint64, hash_map},
    };

    const CONTRACT: Addr = Addr::mock(0);
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
        let inverted_price = !UsdPrice::new_int(price);
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
                post_only: false,
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
            CONTRACT,
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
                post_only: false,
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
            CONTRACT,
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
                post_only: false,
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
        assert_eq!(collections[&TAKER], Uint128::new(500_000_000));
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
            CONTRACT,
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
                post_only: false,
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
            CONTRACT,
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
                post_only: false,
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
            CONTRACT,
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
        let mut state = State::default();

        // Non-vault users: PnL only (no fees), no vault_margin change.
        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(100)),
            (Addr::mock(2), UsdValue::new_int(-200)),
            (Addr::mock(3), UsdValue::ZERO),
        ]);
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        // Positive PnL: user 1 receives 100 × 10^6 base units.
        assert_eq!(payouts.len(), 1);
        assert_eq!(payouts[&Addr::mock(1)], Uint128::new(100_000_000));

        // Negative PnL: user 2 owes 200 × 10^6 base units.
        assert_eq!(collections[&Addr::mock(2)], Uint128::new(200_000_000));

        // Non-vault PnL does not change vault_margin.
        assert_eq!(state.vault_margin, Uint128::ZERO);
    }

    #[test]
    fn settle_pnls_all_payouts() {
        let mut state = State::default();

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(100)),
            (Addr::mock(2), UsdValue::new_int(50)),
        ]);
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert_eq!(payouts.len(), 2);
        assert!(collections.is_empty());

        // Non-vault PnL does not change vault_margin.
        assert_eq!(state.vault_margin, Uint128::ZERO);
    }

    #[test]
    fn settle_pnls_all_collections() {
        let mut state = State::default();

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(-100)),
            (Addr::mock(2), UsdValue::new_int(-200)),
        ]);
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert_eq!(collections.len(), 2);

        // Non-vault PnL does not change vault_margin.
        assert_eq!(state.vault_margin, Uint128::ZERO);
    }

    #[test]
    fn settle_pnls_empty() {
        let mut state = State {
            vault_margin: Uint128::new(500_000_000),
            ..Default::default()
        };

        let pnls = BTreeMap::new();
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert_eq!(state.vault_margin, Uint128::new(500_000_000));
    }

    #[test]
    fn settle_pnls_fees_increase_vault_margin() {
        let mut state = State::default();

        let pnls = BTreeMap::new();
        let fees = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(50)),
            (Addr::mock(2), UsdValue::new_int(100)),
        ]);

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert_eq!(collections.len(), 2);

        // Fees go to vault: vault_margin += 50 + 100 = 150 (in base units).
        assert_eq!(state.vault_margin, Uint128::new(150_000_000));
    }

    #[test]
    fn settle_pnls_vault_pnl_adjusts_margin() {
        let mut state = State {
            vault_margin: Uint128::new(1_000_000_000),
            ..Default::default()
        };

        // Vault profit of $500.
        let pnls = BTreeMap::from([(CONTRACT, UsdValue::new_int(500))]);
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert_eq!(state.vault_margin, Uint128::new(1_500_000_000));
    }

    #[test]
    fn settle_pnls_vault_loss_creates_bad_debt() {
        let mut state = State {
            vault_margin: Uint128::new(100_000_000), // $100
            ..Default::default()
        };

        // Vault loss of $500.
        let pnls = BTreeMap::from([(CONTRACT, UsdValue::new_int(-500))]);
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert_eq!(state.vault_margin, Uint128::ZERO);
        assert_eq!(state.adl_deficit, Uint128::new(400_000_000));
    }

    #[test]
    fn settle_pnls_vault_profit_repays_adl_deficit() {
        let mut state = State {
            vault_margin: Uint128::ZERO,
            adl_deficit: Uint128::new(300_000_000), // $300
            ..Default::default()
        };

        // Vault profit of $500.
        let pnls = BTreeMap::from([(CONTRACT, UsdValue::new_int(500))]);
        let fees = BTreeMap::new();

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        // adl_deficit fully repaid.
        assert_eq!(state.adl_deficit, Uint128::ZERO);
        // Remainder goes to vault_margin: $500 - $300 = $200.
        assert_eq!(state.vault_margin, Uint128::new(200_000_000));
    }

    #[test]
    fn settle_pnls_vault_fees_skipped() {
        let mut state = State {
            vault_margin: Uint128::new(1_000_000_000),
            ..Default::default()
        };

        // Vault's own fees are a no-op (paying yourself).
        let pnls = BTreeMap::new();
        let fees = BTreeMap::from([(CONTRACT, UsdValue::new_int(100))]);

        let (payouts, collections) =
            settle_pnls(pnls, fees, SETTLEMENT_PRICE, &mut state, CONTRACT).unwrap();

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert_eq!(state.vault_margin, Uint128::new(1_000_000_000));
    }

    // =================== Post-only order tests ===============================

    #[test]
    fn post_only_buy_rests_below_best_ask() {
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

        let (payouts, collections, _, order_mutations, order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // No fills — order rests.
        assert!(!taker_state.positions.contains_key(&pair_id()));
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(10));

        // No payouts/collections/mutations.
        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert!(order_mutations.is_empty());
    }

    #[test]
    fn post_only_buy_at_best_ask_rejected() {
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

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
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
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("post-only buy"));
    }

    #[test]
    fn post_only_buy_above_best_ask_rejected() {
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

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(51_000),
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("post-only buy"));
    }

    #[test]
    fn post_only_sell_rests_above_best_bid() {
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

        let (payouts, collections, _, order_mutations, order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(-10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(51_000),
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // No fills — order rests.
        assert!(!taker_state.positions.contains_key(&pair_id()));
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(-10));

        assert!(payouts.is_empty());
        assert!(collections.is_empty());
        assert!(order_mutations.is_empty());
    }

    #[test]
    fn post_only_sell_at_best_bid_rejected() {
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

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(-10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("post-only sell"));
    }

    #[test]
    fn post_only_buy_empty_book_succeeds() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        // No asks placed — empty book.

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (.., order_to_store) = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(10));
    }

    #[test]
    fn post_only_reduce_only_rests() {
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
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            LARGE_COLLATERAL,
            Quantity::new_int(-5),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(51_000),
                post_only: true,
            },
            true,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // Order rests, reduce_only flag preserved.
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(-5));
        assert!(order.reduce_only);
    }

    // ======= Post-only insufficient margin rejected ==========================

    /// Post-only buy with insufficient collateral is rejected by the 0%-fill
    /// margin check inside `store_limit_order`.
    ///
    /// pair: BTC, oracle = $50,000, IMR = 5%
    /// Buy 10 BTC post-only @ $49,000
    ///   margin_to_reserve = |10| * 49,000 * 0.05 = $24,500
    ///   collateral = $1,000 → available = $1,000 < $24,500 → FAILS
    #[test]
    fn post_only_insufficient_margin_rejected() {
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

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            UsdValue::new_int(1_000), // insufficient collateral
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: true,
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        );

        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("insufficient margin for limit order"),
            "expected limit-order margin error, got: {msg}"
        );
    }

    // ======= Self-trade prevention (EXPIRE_MAKER) ========================

    #[test]
    fn self_trade_prevention_expire_maker() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // TAKER has a resting ask at 50,000 (order_id 100).
        place_ask(&mut ctx.storage, TAKER, 50_000, 10, 100);
        // MAKER_A has a resting ask behind it at 50,100 (order_id 101).
        place_ask(&mut ctx.storage, MAKER_A, 50_100, 10, 101);

        // Snapshot taker state after placing the resting order.
        let taker_state_before = USER_STATES.load(&ctx.storage, TAKER).unwrap();
        assert_eq!(taker_state_before.open_order_count, 1);
        let taker_reserved_before = taker_state_before.reserved_margin;

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();

        // Start with the taker state from storage (has the resting order's
        // reserved margin and open_order_count).
        let mut taker_state = taker_state_before.clone();
        let mut oq = test_oracle_querier();
        let mut state = State::default();

        let (_, _, maker_states, order_mutations, _) = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(50_100),
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

        // 1) Taker's own ask was cancelled (mutation = None).
        assert_eq!(order_mutations.len(), 2);
        assert!(
            order_mutations[0].2.is_none(),
            "taker's resting order should be removed"
        );

        // 2) Taker's open_order_count was decremented (the resting order
        //    was cancelled by STP).
        assert_eq!(taker_state.open_order_count, 0);

        // 3) Taker's reserved margin from the cancelled order was released.
        assert!(taker_reserved_before.is_non_zero());
        // The reserved_margin for the cancelled order is fully released.
        // Only trading-fee deductions (from the fill against MAKER_A) remain.
        assert!(taker_state.reserved_margin < taker_reserved_before);

        // 4) Taker DID fill against the second maker's ask at 50,100.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_100));

        // 5) MAKER_A's ask was fully filled (second mutation = None).
        assert!(
            order_mutations[1].2.is_none(),
            "MAKER_A's order should be fully filled"
        );
        assert_eq!(maker_states[&MAKER_A].open_order_count, 0);
    }

    // ======= Vault-as-maker PnL settlement ===================================

    /// Helper: run a two-step trade where the vault (CONTRACT) is the maker on
    /// both legs.
    ///
    /// Step 1: vault ask at `open_price` matched by taker buy → vault opens short.
    /// Step 2: vault bid at `close_price` matched by taker sell → vault closes short.
    ///
    /// Mutates: nothing persisted.
    ///
    /// Returns: the `State` after both trades, plus payouts/collections from the
    /// closing trade.
    fn vault_maker_round_trip(
        initial_vault_margin: Uint128,
        open_price: i128,
        close_price: i128,
        size: i128,
    ) -> (State, BTreeMap<Addr, Uint128>, BTreeMap<Addr, Uint128>) {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // ---- Step 1: vault places ask, taker buys → vault opens short ----

        place_ask(&mut ctx.storage, CONTRACT, open_price, size, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();
        let mut oq = test_oracle_querier();
        let mut state = State {
            vault_margin: initial_vault_margin,
            ..Default::default()
        };

        let (_, _, maker_states, order_mutations, _) = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(open_price),
            LARGE_COLLATERAL,
            Quantity::new_int(size),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        // Persist side-effects so step 2 can load them.
        PAIR_STATES
            .save(&mut ctx.storage, &pair_id(), &pair_state)
            .unwrap();
        USER_STATES
            .save(&mut ctx.storage, TAKER, &taker_state)
            .unwrap();
        for (addr, ms) in &maker_states {
            USER_STATES.save(&mut ctx.storage, *addr, ms).unwrap();
        }
        for (stored_price, order_id, mutation) in order_mutations {
            let key = (pair_id(), stored_price, order_id);
            match mutation {
                Some(order) => ASKS.save(&mut ctx.storage, key, &order).unwrap(),
                None => ASKS.remove(&mut ctx.storage, key).unwrap(),
            }
        }

        // Vault must now have a short position.
        let vault_state = USER_STATES.load(&ctx.storage, CONTRACT).unwrap();
        let vault_pos = vault_state.positions.get(&pair_id()).unwrap();
        assert_eq!(vault_pos.size, Quantity::new_int(-size.abs()));

        // ---- Step 2: vault places bid, taker sells → vault closes short ----

        place_bid(&mut ctx.storage, CONTRACT, close_price, size, 200);

        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = USER_STATES.load(&ctx.storage, TAKER).unwrap();
        let mut oq = test_oracle_querier();

        let (payouts, collections, ..) = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(close_price),
            LARGE_COLLATERAL,
            Quantity::new_int(-size),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            &mut oq,
            SETTLEMENT_PRICE,
            &mut state,
        )
        .unwrap();

        (state, payouts, collections)
    }

    /// Vault opens short at $50,000 then closes at $49,000 → profit of $10,000.
    ///
    /// vault_margin changes from: vault PnL + non-vault fees only.
    /// Non-vault PnL (taker's loss) does NOT change vault_margin — the
    /// losing counterparty pays the winning one directly.
    #[test]
    fn vault_maker_realizes_profit() {
        let initial_margin = Uint128::new(100_000_000_000); // $100,000

        let (state, payouts, collections) =
            vault_maker_round_trip(initial_margin, 50_000, 49_000, 10);

        // vault_margin tracks vault PnL + fee flows only:
        //
        // Step 1 (open at $50,000):
        //   taker fee = |10| × $50,000 × 0.001 = $500 → vault_margin += 500,000,000
        //
        // Step 2 (close at $49,000):
        //   vault PnL = +$10,000 → vault_margin += 10,000,000,000
        //   taker fee = $490 → vault_margin += 490,000,000
        //   taker PnL = -$10,000 → collection (no vault_margin change)
        //
        // Total Δ = 500,000,000 + 10,000,000,000 + 490,000,000 = 10,990,000,000
        assert_eq!(state.vault_margin, Uint128::new(110_990_000_000));

        assert!(state.adl_deficit.is_zero());

        // Vault must not appear in payouts/collections.
        assert!(!payouts.contains_key(&CONTRACT));
        assert!(!collections.iter().any(|(addr, _)| *addr == CONTRACT));
    }

    /// Vault opens short at $50,000 then closes at $51,000 → loss of $10,000.
    /// vault_margin is large enough to absorb the loss entirely.
    #[test]
    fn vault_maker_realizes_loss_no_bad_debt() {
        let initial_margin = Uint128::new(100_000_000_000); // $100,000

        let (state, payouts, collections) =
            vault_maker_round_trip(initial_margin, 50_000, 51_000, 10);

        // vault_margin tracks vault PnL + fee flows only:
        //
        // Step 1 (open at $50,000):
        //   taker fee = $500 → vault_margin += 500,000,000
        //
        // Step 2 (close at $51,000):
        //   taker fee = $510 → vault_margin += 510,000,000
        //   vault PnL = -$10,000 → vault_margin -= 10,000,000,000
        //   taker PnL = +$10,000 → payout (no vault_margin change)
        //
        // Total Δ = 500,000,000 + 510,000,000 - 10,000,000,000 = -8,990,000,000
        assert_eq!(state.vault_margin, Uint128::new(91_010_000_000));

        assert!(state.adl_deficit.is_zero());

        assert!(!payouts.contains_key(&CONTRACT));
        assert!(!collections.iter().any(|(addr, _)| *addr == CONTRACT));
    }

    /// Vault has an existing short position at $50,000. A new taker (MAKER_B)
    /// sells against the vault's bid at $51,000, closing the vault's short at a
    /// loss. vault_margin is only $1,000 — not enough to cover the $10,000 loss,
    /// so the excess flows into adl_deficit.
    ///
    /// Fees are collected first (augmenting vault_margin), then the vault loss
    /// absorbs from the fee-augmented margin.
    #[test]
    fn vault_maker_realizes_loss_with_bad_debt() {
        let mut ctx = MockContext::new()
            .with_sender(MAKER_B)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // Manually set up vault with a short position: -10 @ $50,000.
        let mut vault_state = UserState::default();
        vault_state.positions.insert(pair_id(), Position {
            size: Quantity::new_int(-10),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        USER_STATES
            .save(&mut ctx.storage, CONTRACT, &vault_state)
            .unwrap();

        // Reflect existing OI from the vault's short.
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        pair_state.short_oi = Quantity::new_int(10);
        PAIR_STATES
            .save(&mut ctx.storage, &pair_id(), &pair_state)
            .unwrap();

        // Vault places a resting bid at $51,000.
        place_bid(&mut ctx.storage, CONTRACT, 51_000, 10, 200);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let mut taker_state = UserState::default();
        let mut oq = test_oracle_querier();

        let mut state = State {
            vault_margin: Uint128::new(1_000_000_000), // $1,000
            ..Default::default()
        };

        // MAKER_B sells -10 → matches vault bid at $51,000.
        //   vault: closes short at $51,000 → loss = -$10,000
        //   MAKER_B: opens new short → PnL = 0, fee = |10| × $51,000 × 0.001 = $510
        let (payouts, collections, ..) = _submit_order(
            &ctx.storage,
            MAKER_B,
            CONTRACT,
            &param,
            &pair_param,
            &mut pair_state,
            &mut taker_state,
            &pair_id(),
            UsdPrice::new_int(51_000),
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

        // Fee loop first: MAKER_B fee = $510 → vault_margin += 510,000,000
        //   vault_margin = 1,000,000,000 + 510,000,000 = 1,510,000,000
        //
        // PnL loop: vault loss = $10,000 = 10,000,000,000 base units.
        //   absorbed = min(10,000,000,000, 1,510,000,000) = 1,510,000,000
        //   unabsorbed → adl_deficit = 8,490,000,000
        //   vault_margin → 0
        assert_eq!(state.vault_margin, Uint128::ZERO);
        assert_eq!(state.adl_deficit, Uint128::new(8_490_000_000));

        assert!(!payouts.contains_key(&CONTRACT));
        assert!(!collections.iter().any(|(addr, _)| *addr == CONTRACT));
    }
}
