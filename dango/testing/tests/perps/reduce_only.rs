//! Tests for the reduce-only order invariant: a user's resting reduce-only
//! orders may only move their position toward zero — never grow it, never flip
//! it. The invariant is:
//!
//! > For every (user, pair): the resting reduce-only orders are all on the
//! > position-closing side, and their absolute sizes sum to no more than the
//! > `|position|`.
//!
//! It is enforced by three cooperating layers:
//!
//! - **Placement** (`compute_submit_order_outcome` + the re-size in
//!   `_submit_order`): a new reduce-only order is clamped to the *remaining*
//!   budget after the user's other reduce-only orders; if the sum-clamp leaves
//!   it nothing, the transaction is rejected. A better-priced new order instead
//!   evicts the user's worse-priced ones.
//! - **Dynamic re-size** (`resize_reduce_only_orders`): after every position
//!   change — a fill (as taker or maker), liquidation, or ADL — the user's
//!   resting reduce-only orders are re-clamped to the new position, shrinking or
//!   cancelling the worst by price-time priority.
//! - **Match-time clamp** (`walk_book`): within a single sweep, before the
//!   dynamic re-size has run, each reduce-only maker fill is re-clamped against
//!   the maker's running position.
//!
//! See `dango/perps/src/trade/resize_reduce_only.rs`, `submit_order.rs`, and
//! `dango/order-book/src/matching_engine.rs`.

use {
    crate::{default_pair_param, default_param, register_oracle_prices},
    dango_order_book::{
        Dimensionless, OrderId, OrderKind, OrderRemoved, OrderResized, PairId, Quantity,
        QueryOrdersByUserResponseItem, ReasonForOrderRemoval, TimeInForce, UsdPrice,
    },
    dango_testing::{TestAccount, TestOption, TestSuiteNaive, pair_id, setup_test_naive},
    dango_types::{
        constants::usdc,
        perps::{self, PairParam, PairState, UserState},
    },
    grug_math::Uint128,
    grug_types::{
        Addr, Addressable, CheckedContractEvent, Coins, JsonDeExt, QuerierExt, ResultExt,
        SearchEvent, btree_map,
    },
    std::collections::BTreeMap,
};

/// $50,000 USDC (6 decimals) — generous so margin never clouds the
/// position-size assertions, while staying within each test account's balance.
const DEPOSIT: u128 = 50_000_000_000;

// --------------------------------- helpers -----------------------------------

/// Deposit `amount` (USDC base units) of margin for `account`.
async fn deposit(
    suite: &mut TestSuiteNaive,
    account: &mut TestAccount,
    contract: Addr,
    amount: u128,
) {
    suite
        .execute(
            account,
            contract,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(amount)).unwrap(),
        )
        .await
        .should_succeed();
}

/// Submit a post-only limit order, which rests on the book without matching.
/// Positive `size` is a bid (buy), negative an ask (sell).
async fn rest_limit(
    suite: &mut TestSuiteNaive,
    account: &mut TestAccount,
    contract: Addr,
    pair: &PairId,
    size: i128,
    price: i128,
    reduce_only: bool,
) {
    suite
        .execute(
            account,
            contract,
            &post_only(pair, size, price, reduce_only),
            Coins::new(),
        )
        .await
        .should_succeed();
}

/// Build a post-only limit order request (positive `size` is a bid).
fn post_only(pair: &PairId, size: i128, price: i128, reduce_only: bool) -> perps::ExecuteMsg {
    perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
        pair_id: pair.clone(),
        size: Quantity::new_int(size),
        kind: OrderKind::Limit {
            limit_price: UsdPrice::new_int(price),
            time_in_force: TimeInForce::PostOnly,
            client_order_id: None,
        },
        reduce_only,
        tp: None,
        sl: None,
    }))
}

/// Build a market order request with a generous 50% slippage tolerance.
fn market(pair: &PairId, size: i128, reduce_only: bool) -> perps::ExecuteMsg {
    perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
        pair_id: pair.clone(),
        size: Quantity::new_int(size),
        kind: OrderKind::Market {
            max_slippage: Dimensionless::new_percent(50),
        },
        reduce_only,
        tp: None,
        sl: None,
    }))
}

/// Submit a market order that is expected to (at least partially) fill.
async fn market_fill(
    suite: &mut TestSuiteNaive,
    account: &mut TestAccount,
    contract: Addr,
    pair: &PairId,
    size: i128,
    reduce_only: bool,
) {
    suite
        .execute(
            account,
            contract,
            &market(pair, size, reduce_only),
            Coins::new(),
        )
        .await
        .should_succeed();
}

fn user_state(suite: &TestSuiteNaive, contract: Addr, user: Addr) -> UserState {
    suite
        .query_wasm_smart(contract, perps::QueryUserStateRequest { user })
        .should_succeed()
        .unwrap()
}

/// The user's signed position size in `pair`, or `None` if the user has no
/// position there (a position closed to exactly zero is removed from the map).
fn position(suite: &TestSuiteNaive, contract: Addr, user: Addr, pair: &PairId) -> Option<Quantity> {
    user_state(suite, contract, user)
        .positions
        .get(pair)
        .map(|p| p.size)
}

fn open_orders(
    suite: &TestSuiteNaive,
    contract: Addr,
    user: Addr,
) -> BTreeMap<OrderId, QueryOrdersByUserResponseItem> {
    suite
        .query_wasm_smart(contract, perps::QueryOrdersByUserRequest { user })
        .should_succeed()
}

/// The user's single resting order at `price`, if any.
fn order_at(
    suite: &TestSuiteNaive,
    contract: Addr,
    user: Addr,
    price: i128,
) -> Option<QueryOrdersByUserResponseItem> {
    open_orders(suite, contract, user)
        .into_values()
        .find(|o| o.limit_price == UsdPrice::new_int(price))
}

/// The pair's state, carrying `long_oi` / `short_oi`. Open interest is a strong
/// cross-check on the clamp: a fill that wrongly opened a position would inflate
/// OI, and after every fill the invariant `long_oi == short_oi` must hold.
fn pair_state(suite: &TestSuiteNaive, contract: Addr, pair: &PairId) -> PairState {
    suite
        .query_wasm_smart(contract, perps::QueryPairStateRequest {
            pair_id: pair.clone(),
        })
        .should_succeed()
        .unwrap()
}

// ----------------------------- placement clamp -------------------------------

/// Placement rejects a reduce-only order whose size, summed with the user's
/// existing reduce-only orders, would exceed the position. This is the
/// defense-in-depth that kills the "many copies of a 1-unit reduce-only order"
/// attack at the source: the second copy can never rest.
///
/// A maker opens a 1-unit long and rests a reduce-only sell of 1 (the whole
/// position). A second reduce-only sell of 1 is rejected — there is no position
/// budget left for it. The single resting order then closes the long normally.
#[tokio::test]
async fn reduce_only_placement_rejects_when_sum_exceeds_position() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    // user3 rests an ask of 1 so the maker can open a long.
    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -1,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 1, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(1))
    );

    // First reduce-only sell of 1 rests — it exactly fills the position budget.
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -1,
        2_001,
        true,
    )
    .await;

    // Second reduce-only sell of 1 is rejected: the sum (2) would exceed the
    // position (1). It is a post-only order — i.e. one that *would* rest — so
    // the sum-clamp actually fires (an IOC/market order would just not fill).
    suite
        .execute(
            &mut accounts.user2,
            perps,
            &post_only(&pair, -1, 2_002, true),
            Coins::new(),
        )
        .await
        .should_fail_with_error("reduce-only order would exceed position size");

    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1,
        "only the first reduce-only order rests; the second was rejected"
    );

    // The one valid order closes the long normally.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 1, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(1))
    );
    assert!(open_orders(&suite, perps, accounts.user2.address()).is_empty());

    let ps = pair_state(&suite, perps, &pair);
    assert_eq!(ps.long_oi, Quantity::new_int(1));
    assert_eq!(ps.short_oi, Quantity::new_int(1));
}

/// Short-maker mirror of [`reduce_only_placement_rejects_when_sum_exceeds_position`]:
/// a 1-unit short with reduce-only *buys*. The second buy is rejected.
#[tokio::test]
async fn reduce_only_placement_rejects_short_maker() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    // user3 rests a bid of 1 so the maker can open a short.
    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        1,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, -1, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-1))
    );

    // First reduce-only buy of 1 rests; the second is rejected.
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        1,
        1_999,
        true,
    )
    .await;
    suite
        .execute(
            &mut accounts.user2,
            perps,
            &post_only(&pair, 1, 1_998, true),
            Coins::new(),
        )
        .await
        .should_fail_with_error("reduce-only order would exceed position size");

    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1
    );

    market_fill(&mut suite, &mut accounts.user1, perps, &pair, -1, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(-1))
    );
}

/// A better-priced new reduce-only order evicts the user's own worse-priced
/// one: re-quoting at a price more likely to fill. The new order takes the whole
/// budget; the stale one is cancelled.
///
/// Maker is long 5 with a reduce-only sell of 5 resting at 2005. A new
/// reduce-only sell of 5 at 2002 (better — a lower ask fills first) takes the
/// whole 5-unit budget, so the 2005 order is cancelled.
#[tokio::test]
async fn reduce_only_placement_better_price_replaces_worse() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -5,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 5, false).await;

    // Worse-priced reduce-only sell first, then a better-priced one.
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_005,
        true,
    )
    .await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_002,
        true,
    )
    .await;

    // Only the better-priced order survives; the worse one was evicted.
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1
    );
    assert_eq!(
        order_at(&suite, perps, accounts.user2.address(), 2_002)
            .unwrap()
            .size,
        Quantity::new_int(-5)
    );
    assert!(order_at(&suite, perps, accounts.user2.address(), 2_005).is_none());
}

/// A new reduce-only order is shrunk to the budget left by the user's existing
/// reduce-only orders, rather than rejected, when that budget is positive.
///
/// Maker is long 10 with a reduce-only sell of 6 at 2001 (better). A new
/// reduce-only sell of 10 at 2002 (worse) is shrunk to 4 — the budget left after
/// the 6 — and both rest.
#[tokio::test]
async fn reduce_only_placement_partial_budget_shrinks_new() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 10, false).await;

    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -6,
        2_001,
        true,
    )
    .await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -10,
        2_002,
        true,
    )
    .await;

    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        2
    );
    assert_eq!(
        order_at(&suite, perps, accounts.user2.address(), 2_001)
            .unwrap()
            .size,
        Quantity::new_int(-6)
    );
    assert_eq!(
        order_at(&suite, perps, accounts.user2.address(), 2_002)
            .unwrap()
            .size,
        Quantity::new_int(-4),
        "the worse order is shrunk to the remaining budget (10 - 6)"
    );
}

// --------------------------- dynamic re-sizing -------------------------------

/// A reduce-only order is re-sized down when its owner's position shrinks via a
/// separate (taker) fill — *before* any taker reaches it. With the order then
/// already right-sized, the match-time clamp has nothing left to do: a taker
/// buying exactly the resized amount fills it in full, with no mid-sweep skip.
/// This is the layering thesis — eager re-sizing makes the match-time clamp
/// redundant outside a single sweep.
#[tokio::test]
async fn reduce_only_resize_on_taker_fill_then_clamp_is_noop() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;

    // Maker (user2) opens a 10-unit long and rests a reduce-only sell of 10.
    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 10, false).await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -10,
        2_002,
        true,
    )
    .await;

    let reserved_before = user_state(&suite, perps, accounts.user2.address()).reserved_margin;

    // Maker shrinks the long to 4 by selling 6 into user4's bid.
    rest_limit(
        &mut suite,
        &mut accounts.user4,
        perps,
        &pair,
        6,
        2_001,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, -6, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(4))
    );

    // The reduce-only order was re-sized 10 -> 4 the instant the position
    // shrank, with reserved margin released proportionally.
    let resting = order_at(&suite, perps, accounts.user2.address(), 2_002).unwrap();
    assert_eq!(
        resting.size,
        Quantity::new_int(-4),
        "re-sized to the new position"
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1
    );
    assert!(
        user_state(&suite, perps, accounts.user2.address()).reserved_margin < reserved_before,
        "shrinking the order released part of its reserved margin"
    );

    // A taker buys exactly 4: the order is already right-sized, so it fills in
    // full and is removed — the match-time clamp had nothing to clamp.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 4, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(4))
    );
    assert!(open_orders(&suite, perps, accounts.user2.address()).is_empty());

    let ps = pair_state(&suite, perps, &pair);
    assert_eq!(ps.long_oi, ps.short_oi, "OI must net");
    assert_eq!(ps.long_oi, Quantity::new_int(10));
}

/// A reduce-only order is re-sized when its owner's position shrinks because one
/// of their *other*, non-reduce-only orders is filled (the owner is the maker,
/// not the taker). Also asserts the `OrderResized` event.
///
/// Maker is long 10, resting a non-reduce-only sell of 6 (at 2001) and a
/// reduce-only sell of 10 (at 2003). A taker buys 6, filling only the non-RO
/// order; the maker drops to long 4, and its reduce-only order is re-sized to 4.
#[tokio::test]
async fn reduce_only_resize_on_maker_fill_shrinks_order() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 10, false).await;

    // Non-reduce-only sell of 6 (senior) and reduce-only sell of 10.
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -6,
        2_001,
        false,
    )
    .await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -10,
        2_003,
        true,
    )
    .await;

    // Taker buys 6 — fills only the non-RO order, dropping the maker to long 4.
    let events = suite
        .execute(
            &mut accounts.user1,
            perps,
            &market(&pair, 6, false),
            Coins::new(),
        )
        .await
        .should_succeed()
        .events;

    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(4))
    );

    // The non-RO order filled and was removed; the reduce-only order was re-sized
    // 10 -> 4.
    assert!(order_at(&suite, perps, accounts.user2.address(), 2_001).is_none());
    assert_eq!(
        order_at(&suite, perps, accounts.user2.address(), 2_003)
            .unwrap()
            .size,
        Quantity::new_int(-4)
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1
    );

    // The re-size emitted an `OrderResized` event carrying old and new size.
    let resized = events
        .search_event::<CheckedContractEvent>()
        .with_predicate(|e| e.ty == "order_resized")
        .take()
        .all()
        .into_iter()
        .map(|e| e.event.data.deserialize_json::<OrderResized>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(resized.len(), 1, "exactly one order was shrunk");
    assert_eq!(resized[0].user, accounts.user2.address());
    assert_eq!(resized[0].old_size, Quantity::new_int(-10));
    assert_eq!(resized[0].new_size, Quantity::new_int(-4));
}

/// When the owner's position flips, every reduce-only order is on the wrong side
/// and is cancelled (not merely skipped). Asserts the `OrderRemoved` event with
/// the `ReduceOnlyResized` reason, and that a taker then finds no liquidity.
///
/// Maker is long 10 with a reduce-only sell of 10 resting, then flips to a 5-unit
/// short. The reduce-only sell would now grow the short, so it is cancelled.
#[tokio::test]
async fn reduce_only_resize_cancels_on_flip() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 10, false).await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -10,
        2_002,
        true,
    )
    .await;

    // Maker flips to a 5-unit short by selling 15 into user4's bid.
    rest_limit(
        &mut suite,
        &mut accounts.user4,
        perps,
        &pair,
        15,
        2_001,
        false,
    )
    .await;
    let events = suite
        .execute(
            &mut accounts.user2,
            perps,
            &market(&pair, -15, false),
            Coins::new(),
        )
        .await
        .should_succeed()
        .events;

    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-5))
    );

    // The reduce-only sell was cancelled — not left resting.
    assert!(
        open_orders(&suite, perps, accounts.user2.address()).is_empty(),
        "the wrong-side reduce-only order is cancelled on the flip"
    );

    let removed = events
        .search_event::<CheckedContractEvent>()
        .with_predicate(|e| e.ty == "order_removed")
        .take()
        .all()
        .into_iter()
        .map(|e| e.event.data.deserialize_json::<OrderRemoved>().unwrap())
        .filter(|e| e.reason == ReasonForOrderRemoval::ReduceOnlyResized)
        .collect::<Vec<_>>();
    assert_eq!(
        removed.len(),
        1,
        "one reduce-only order cancelled by re-sizing"
    );
    assert_eq!(removed[0].user, accounts.user2.address());

    // A taker now finds no resting ask to hit.
    suite
        .execute(
            &mut accounts.user1,
            perps,
            &market(&pair, 10, false),
            Coins::new(),
        )
        .await
        .should_fail_with_error("no liquidity at acceptable price");
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-5)),
        "the short is untouched"
    );
}

/// Within a single sweep, an earlier non-reduce-only order flips the maker past
/// zero; the reduce-only order reached later is skipped by the match-time clamp,
/// then cancelled by the post-sweep re-size.
///
/// Maker is long 5, resting a non-reduce-only sell of 10 (senior) and a
/// reduce-only sell of 5. A taker buys 20: the non-RO order fills 10, flipping
/// the maker to a 5-unit short; the reduce-only order is inert and is cancelled.
#[tokio::test]
async fn reduce_only_resize_after_earlier_order_flips_maker() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -5,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 5, false).await;

    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -10,
        2_001,
        false,
    )
    .await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_002,
        true,
    )
    .await;

    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 20, false).await;

    // A flipped the maker 5 -> -5; B (reduce-only) was skipped then cancelled.
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-5))
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(10))
    );
    assert!(
        open_orders(&suite, perps, accounts.user2.address()).is_empty(),
        "the inert reduce-only order is cancelled by re-sizing"
    );

    let ps = pair_state(&suite, perps, &pair);
    assert_eq!(ps.long_oi, Quantity::new_int(10));
    assert_eq!(ps.short_oi, Quantity::new_int(10));
}

/// Several of the SAME maker's orders are matched in one sweep, where an earlier
/// non-reduce-only order reduces the position before a later reduce-only order is
/// reached. The reduce-only order's match-time clamp reflects the earlier fill
/// (filling only what remains), and the post-sweep re-size cancels its now-inert
/// remainder.
///
/// Maker is long 10, resting a non-reduce-only sell of 6 (senior) and a
/// reduce-only sell of 10. A taker buys 16: the non-RO closes 6 (-> 4), the
/// reduce-only clamps to 4 and closes it (-> 0), and its -6 remainder is then
/// cancelled.
#[tokio::test]
async fn reduce_only_clamp_reflects_earlier_fill_in_same_sweep() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 10, false).await;

    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -6,
        2_001,
        false,
    )
    .await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -10,
        2_002,
        true,
    )
    .await;

    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 16, false).await;

    // Maker ends flat; the reduce-only order clamped to the reduced position
    // (filling 4) and its -6 remainder was cancelled — not left resting.
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(10))
    );
    assert!(
        open_orders(&suite, perps, accounts.user2.address()).is_empty(),
        "the inert remainder is cancelled by re-sizing"
    );

    let ps = pair_state(&suite, perps, &pair);
    assert_eq!(ps.long_oi, Quantity::new_int(10));
    assert_eq!(ps.short_oi, Quantity::new_int(10));
}

/// One taker sweeps reduce-only orders from two different makers; each must clamp
/// to its OWN position. One maker's order was already re-sized down when that
/// maker's position shrank, so the sweep sees the correct, smaller order.
///
/// user2 and user3 each open a 5-unit long and rest a reduce-only sell of 5.
/// user3 then shrinks to 1 — re-sizing its order to 1. A taker buys 10: user2
/// closes its full 5, user3 closes its 1; both end flat.
#[tokio::test]
async fn reduce_only_resize_per_maker_across_two_makers() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;

    // user4 sells 10; user2 and user3 each buy 5.
    rest_limit(
        &mut suite,
        &mut accounts.user4,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 5, false).await;
    market_fill(&mut suite, &mut accounts.user3, perps, &pair, 5, false).await;

    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_001,
        true,
    )
    .await;
    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -5,
        2_001,
        true,
    )
    .await;

    // user3 shrinks its long to 1 by selling 4 into user4's bid — its reduce-only
    // order is re-sized 5 -> 1 at that moment.
    rest_limit(
        &mut suite,
        &mut accounts.user4,
        perps,
        &pair,
        4,
        1_999,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user3, perps, &pair, -4, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user3.address(), &pair),
        Some(Quantity::new_int(1))
    );
    assert_eq!(
        order_at(&suite, perps, accounts.user3.address(), 2_001)
            .unwrap()
            .size,
        Quantity::new_int(-1),
        "user3's reduce-only order was re-sized to its new position"
    );

    // Taker buys 10: user2's 5 (senior) then user3's 1.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 10, false).await;

    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None
    );
    assert_eq!(
        position(&suite, perps, accounts.user3.address(), &pair),
        None
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(6))
    );
    assert!(open_orders(&suite, perps, accounts.user2.address()).is_empty());
    assert!(open_orders(&suite, perps, accounts.user3.address()).is_empty());

    let ps = pair_state(&suite, perps, &pair);
    assert_eq!(ps.long_oi, Quantity::new_int(6));
    assert_eq!(ps.short_oi, Quantity::new_int(6));
}

/// Re-sizing only ever shrinks/cancels — it never grows a reduce-only order. When
/// the position grows, the order is left exactly as it was (no resurrection).
///
/// Maker is long 5 with a reduce-only sell of 5; the maker buys 5 more (long 10).
/// The order stays at 5.
#[tokio::test]
async fn reduce_only_position_growth_leaves_order() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -5,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 5, false).await;
    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_002,
        true,
    )
    .await;

    // Maker grows the long to 10 by buying 5 from user4.
    rest_limit(
        &mut suite,
        &mut accounts.user4,
        perps,
        &pair,
        -5,
        2_001,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 5, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(10))
    );

    // The reduce-only order is untouched.
    assert_eq!(
        order_at(&suite, perps, accounts.user2.address(), 2_002)
            .unwrap()
            .size,
        Quantity::new_int(-5),
        "growth never grows or resurrects the order"
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1
    );
}

// ------------------------------- regression ----------------------------------

/// Regression: when the position fully covers the reduce-only order, the order
/// fills in full and is removed — the clamp must not under-fill it.
#[tokio::test]
async fn reduce_only_fills_fully_when_position_sufficient() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -10,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 10, false).await;

    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_002,
        true,
    )
    .await;

    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 5, false).await;

    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(5))
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(5))
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        0
    );
}

/// Regression: the clamp applies ONLY to reduce-only orders. A regular resting
/// order may still flip the maker's position — normal trading behavior.
#[tokio::test]
async fn non_reduce_only_order_still_flips_position() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -1,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 1, false).await;

    rest_limit(
        &mut suite,
        &mut accounts.user2,
        perps,
        &pair,
        -5,
        2_002,
        false,
    )
    .await;

    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 5, false).await;

    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-4)),
        "a non-reduce-only order may flip the position"
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(5))
    );
}

// ----------------------------- liquidation / ADL -----------------------------

/// A reduce-only order is re-sized when its owner's position is forcibly reduced
/// as an ADL counter-party during someone else's liquidation. This is the
/// liquidation-path trigger for dynamic re-sizing.
///
/// Carol (user3) is long 5 with a reduce-only sell of 5 resting far out of the
/// money. user1 holds a 3-unit short that goes underwater and is liquidated;
/// with no in-range book liquidity, all 3 are ADL'd against Carol's long, taking
/// it to 2. Carol's reduce-only order is re-sized 5 -> 2.
#[tokio::test]
async fn reduce_only_resize_on_adl_counterparty() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    // Widen the price band so Carol's far-OTM reduce-only ask can be placed and,
    // crucially, so the liquidation's market buy skips it (it sits above the
    // oracle-clamped target price) and falls through to ADL.
    suite
        .execute(
            &mut accounts.owner,
            perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: default_param(),
                pair_params: btree_map! {
                    pair.clone() => PairParam {
                        max_limit_price_deviation: Dimensionless::new_permille(999),
                        ..default_pair_param()
                    },
                },
            }),
            Coins::new(),
        )
        .await
        .should_succeed();

    // Carol (user3) and user4 are well funded; user1 (the short) is funded just
    // enough to open and be liquidatable when the oracle rises.
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user1, perps, 700_000_000).await;

    // Carol rests a bid of 5; user1 sells 3 and user4 sells 2 into it, leaving
    // Carol long 5, user1 short 3, user4 short 2 (OI balanced).
    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        5,
        2_000,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, -3, false).await;
    market_fill(&mut suite, &mut accounts.user4, perps, &pair, -2, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user3.address(), &pair),
        Some(Quantity::new_int(5))
    );

    // Carol rests a reduce-only sell of 5 far out of the money.
    rest_limit(
        &mut suite,
        &mut accounts.user3,
        perps,
        &pair,
        -5,
        3_900,
        true,
    )
    .await;

    // Oracle rises: user1's short is now underwater.
    register_oracle_prices(&mut suite, &mut accounts, 2_300).await;

    // Liquidate user1. No in-range asks (Carol's is at 3,900, above the oracle
    // target), so all 3 are ADL'd against Carol's long (5 -> 2).
    suite
        .execute(
            &mut accounts.owner,
            perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: accounts.user1.address(),
            }),
            Coins::new(),
        )
        .await
        .should_succeed();

    assert_eq!(
        position(&suite, perps, accounts.user3.address(), &pair),
        Some(Quantity::new_int(2)),
        "Carol's long was reduced to 2 by ADL"
    );

    // Carol's reduce-only order was re-sized to her new position.
    assert_eq!(
        order_at(&suite, perps, accounts.user3.address(), 3_900)
            .unwrap()
            .size,
        Quantity::new_int(-2),
        "the reduce-only order tracks the ADL'd-down position"
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user3.address()).open_order_count,
        1
    );
}
