//! Tests for the reduce-only order invariant: a reduce-only order may only
//! move the maker's position toward zero — it must never grow the position or
//! flip it to the opposite side.
//!
//! The invariant is enforced at placement time (the order is clamped to the
//! position when submitted) AND, as of the match-time clamp, every time a
//! resting reduce-only maker order is matched. See `match_order` in
//! `dango/perps/src/trade/submit_order.rs` and `walk_book` in
//! `dango/order-book/src/matching_engine.rs`.

use {
    crate::register_oracle_prices,
    dango_order_book::{
        Dimensionless, OrderId, OrderKind, PairId, Quantity, QueryOrdersByUserResponseItem,
        TimeInForce, UsdPrice,
    },
    dango_testing::{TestAccount, TestOption, TestSuiteNaive, pair_id, setup_test_naive},
    dango_types::{
        constants::usdc,
        perps::{self, UserState},
    },
    grug_math::Uint128,
    grug_types::{Addr, Addressable, Coins, QuerierExt, ResultExt},
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
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
            })),
            Coins::new(),
        )
        .await
        .should_succeed();
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(size),
                kind: OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only,
                tp: None,
                sl: None,
            })),
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

// ---------------------------------- tests ------------------------------------

/// Attack reproduction (defanged: 1 unit, 3 orders instead of 1 BTC / 100
/// orders).
///
/// A maker opens a 1-unit long, then rests three reduce-only sells of 1 unit
/// each. Each is clamped to the position (1) at placement, but the maker's
/// position can change after they rest, and a single taker can sweep all
/// three. The taker buys 3.
///
/// Buggy behavior (v0.21): the resting clamp was never re-checked at match
/// time, and the maker side of a fill happily decomposed an over-large fill
/// into a closing AND an opening portion. So all three orders fully filled:
/// order 1 closed the long (1 -> 0), then orders 2 and 3 OPENED a short
/// (0 -> -1 -> -2). The maker's 1-unit long was flipped into a 2-unit short
/// with no margin check — the core of the exploit. The taker received 3.
///
/// Correct behavior (match-time clamp): each reduce-only fill is re-clamped to
/// the maker's current position. Order 1 closes the long (1 -> 0); orders 2
/// and 3 have nothing left to close, so they are clamped to zero and skipped,
/// left resting. The maker ends flat (never short) and the taker receives only
/// 1. (Cancelling the two now-inert resting orders is the job of the separate
/// dynamic re-sizing work; this PR leaves them on the book.)
#[tokio::test]
async fn reduce_only_maker_position_flip_attack() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    // user3 rests an ask of 1 @ $2,000 so the maker can open a long.
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

    // Maker (user2) market-buys 1 -> opens a 1-unit long.
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, 1, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(1)),
        "maker should be 1 long after opening"
    );

    // Maker rests three reduce-only sells of 1 each @ 2001/2002/2003. Each is
    // clamped to the position (1) at placement and rests at -1.
    for price in [2_001, 2_002, 2_003] {
        rest_limit(
            &mut suite,
            &mut accounts.user2,
            perps,
            &pair,
            -1,
            price,
            true,
        )
        .await;
    }
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(1)),
        "resting orders must not change the position"
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        3,
        "maker should have 3 resting reduce-only sells"
    );

    // Taker (user1) market-buys 3 -> sweeps the reduce-only sells.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 3, false).await;

    // The maker's 1-unit long was closed to flat — NOT flipped into a short.
    // (v0.21 produced a -2 short here.)
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None,
        "maker should be flat — never flipped short"
    );

    // The taker only got the 1 unit the maker could actually close.
    // (v0.21 over-filled the taker to a 3-unit long.)
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(1)),
        "taker should receive only what the maker could close"
    );

    // The two reduce-only orders that had nothing left to close are skipped but
    // remain resting. This PR intentionally does not cancel them — dynamic
    // re-sizing (the follow-up) will. When that lands, this assertion is
    // expected to change (the orders should be cancelled), so it doubles as a
    // reminder.
    let resting = open_orders(&suite, perps, accounts.user2.address());
    assert_eq!(
        resting.len(),
        2,
        "the two unfilled reduce-only orders still rest (until dynamic re-sizing removes them)"
    );
    assert!(
        resting.values().all(|o| o.reduce_only),
        "the resting orders should be the two reduce-only sells"
    );
    let prices: Vec<_> = resting.values().map(|o| o.limit_price).collect();
    assert!(prices.contains(&UsdPrice::new_int(2_002)));
    assert!(prices.contains(&UsdPrice::new_int(2_003)));
}

/// Several of the SAME maker's orders are matched by a single taker, where an
/// earlier (non-reduce-only) order reduces the position before a later
/// reduce-only order is reached. The later order's clamp must reflect the
/// earlier fill, not the stale pre-sweep position.
///
/// Maker is long 10 and rests:
/// - order A: a non-reduce-only sell of 6 @ 2001 (fills first),
/// - order B: a reduce-only sell of 10 @ 2002 (clamped to 10 at placement).
///
/// A taker buys 16. Order A closes 6 (10 -> 4). Order B must then clamp to the
/// reduced position (4), filling only 4 (4 -> 0) — NOT to the pre-sweep
/// position (10), which would flip the maker to a 6-unit short. This is the
/// case that the per-walk fill tracking (`maker_fill_deltas`) exists to handle.
#[tokio::test]
async fn reduce_only_clamp_reflects_earlier_fill_in_same_sweep() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    // Maker (user2) opens a 10-unit long against user3's ask.
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

    // Order A (non-reduce-only sell 6 @ 2001) and order B (reduce-only sell 10
    // @ 2002). A is the more senior ask (lower price), so it fills first.
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

    // Taker (user1) buys 16, sweeping A then B.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 16, false).await;

    // A closed 6 (10 -> 4); B clamped to the reduced position and closed 4
    // (4 -> 0). Maker ends flat. A naive clamp against the pre-sweep position
    // (10) would have flipped the maker to a 6-unit short.
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None,
        "maker should be flat — the reduce-only order clamped to the reduced position"
    );

    // Taker bought 6 (from A) + 4 (from B) = 10.
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(10)),
        "taker filled 6 from A and 4 from B"
    );

    // Order A fully filled and was removed; order B has its (now inert)
    // remainder resting.
    let resting = open_orders(&suite, perps, accounts.user2.address());
    assert_eq!(
        resting.len(),
        1,
        "only the reduce-only order's remainder rests"
    );
    assert!(resting.values().all(|o| o.reduce_only));
}

/// A reduce-only order rests, then the maker's position shrinks (via a separate
/// trade) before a taker reaches the order. The order's resting size is now
/// stale (larger than the position); the match-time clamp must cap the fill to
/// the current, smaller position.
///
/// Maker is long 10 with a reduce-only sell of 10 resting. The maker then sells
/// 7 (non-reduce-only) to a third party, leaving a 3-unit long. A taker buys
/// 10 against the stale reduce-only order, which must close only 3 (3 -> 0),
/// not 10 (which would flip the maker to a 7-unit short).
#[tokio::test]
async fn reduce_only_clamps_to_stale_shrunken_position() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;

    // Maker (user2) opens a 10-unit long against user3's ask.
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

    // Maker rests a reduce-only sell of 10 (clamped to the position, 10).
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

    // Maker shrinks the long to 3 by selling 7 to user4 (non-reduce-only). This
    // does not touch the resting reduce-only ask, which is now stale at 10.
    rest_limit(
        &mut suite,
        &mut accounts.user4,
        perps,
        &pair,
        7,
        2_001,
        false,
    )
    .await;
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, -7, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(3)),
        "maker long should be reduced to 3"
    );

    // Taker (user1) buys 10 against the stale reduce-only ask.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 10, false).await;

    // The order closed only the 3 units the maker still had — not its stale
    // resting size of 10 (which would have flipped the maker to a 7 short).
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        None,
        "maker should be flat — clamped to the shrunken position"
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(3)),
        "taker received only the 3 the maker could close"
    );
}

/// A reduce-only order whose maker has since flipped to the SAME side as the
/// order is inert: it can only reduce, and there is nothing to reduce in the
/// closing direction, so it must not fill at all.
///
/// Maker is long 10 with a reduce-only sell of 10 resting, then flips to a
/// 5-unit short (selling 15 to a third party). The resting sell would now grow
/// the short, so the match-time clamp drops it to zero and skips it: a taker
/// buying against it finds no fillable liquidity, and the maker stays short 5.
#[tokio::test]
async fn reduce_only_inert_when_position_flipped() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user4, perps, DEPOSIT).await;

    // Maker (user2) opens a 10-unit long against user3's ask.
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

    // Maker rests a reduce-only sell of 10.
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

    // Maker flips to a 5-unit short by selling 15 to user4 (non-reduce-only:
    // closes the 10 long and opens a 5 short).
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
    market_fill(&mut suite, &mut accounts.user2, perps, &pair, -15, false).await;
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-5)),
        "maker should be 5 short after flipping"
    );

    // Taker (user1) tries to buy against the now-inert reduce-only ask. It is
    // the only resting ask, and it is skipped, so the market order finds no
    // liquidity and is rejected.
    suite
        .execute(
            &mut accounts.user1,
            perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .await
        .should_fail_with_error("no liquidity at acceptable price");

    // The maker's short is untouched, and the inert order is still on the book.
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-5)),
        "maker short must be unchanged — the reduce-only order cannot grow it"
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        1,
        "the inert reduce-only order is left resting"
    );
}

/// Regression: when the position fully covers the reduce-only order, the order
/// fills in full and is removed — the clamp must not under-fill it.
///
/// Maker is long 10 with a reduce-only sell of 5 resting. A taker buys 5; the
/// order fills completely (10 -> 5 long) and is removed.
#[tokio::test]
async fn reduce_only_fills_fully_when_position_sufficient() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    // Maker (user2) opens a 10-unit long against user3's ask.
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

    // Maker rests a reduce-only sell of 5 (well within the 10 long).
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

    // Taker (user1) buys 5.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 5, false).await;

    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(5)),
        "maker long reduced from 10 to 5"
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(5)),
        "taker bought 5"
    );
    assert_eq!(
        user_state(&suite, perps, accounts.user2.address()).open_order_count,
        0,
        "the reduce-only order fully filled and was removed"
    );
}

/// Regression: the clamp applies ONLY to reduce-only orders. A regular
/// (non-reduce-only) resting order may still flip the maker's position, which
/// is normal trading behavior.
///
/// Maker is long 1 with a regular sell of 5 resting. A taker buys 5; the order
/// closes the 1 long and opens a 4 short — exactly as it should.
#[tokio::test]
async fn non_reduce_only_order_still_flips_position() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, 2_000).await;

    let perps = contracts.perps;
    let pair = pair_id();

    deposit(&mut suite, &mut accounts.user1, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user2, perps, DEPOSIT).await;
    deposit(&mut suite, &mut accounts.user3, perps, DEPOSIT).await;

    // Maker (user2) opens a 1-unit long against user3's ask.
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

    // Maker rests a REGULAR (non-reduce-only) sell of 5.
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

    // Taker (user1) buys 5.
    market_fill(&mut suite, &mut accounts.user1, perps, &pair, 5, false).await;

    // The regular order closed the 1 long and opened a 4 short — not clamped.
    assert_eq!(
        position(&suite, perps, accounts.user2.address(), &pair),
        Some(Quantity::new_int(-4)),
        "a non-reduce-only order may flip the position"
    );
    assert_eq!(
        position(&suite, perps, accounts.user1.address(), &pair),
        Some(Quantity::new_int(5)),
        "taker bought 5"
    );
}
