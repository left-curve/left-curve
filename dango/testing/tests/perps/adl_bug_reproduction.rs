//! # ADL Bankruptcy Price Bug — Reproduction Test
//!
//! ## The bug
//!
//! The liquidation engine computes the bankruptcy price (used for ADL fills)
//! **after** book matching instead of **before**, and applies no price limit
//! (`target_price`) to the liquidation market order. When resting orders at
//! absurd prices exist on the book, the market order sweeps through them,
//! destroying the liquidated user's equity. The recomputed bankruptcy price
//! becomes an extreme negative number, and ADL counter-parties are forced to
//! close at that price — suffering massive unjust losses.
//!
//! ## The testnet incident
//!
//! User `0xe6ed` held a SHORT ~4,774 ETH position (entry ~$2,040) and was
//! liquidated three times. The on-chain book contained resting asks at absurd
//! prices ($10M, $1M, $68K, etc.). The liquidation market buy swept through
//! all of them, generating ~$11.8M in realized losses. The bankruptcy price
//! was then recomputed from the devastated equity, producing ADL prices of
//! **-$45,763** and **-$9,804**. Counter-parties (profitable longs) were
//! forced to sell at these negative prices, losing a combined ~$19.5M.
//!
//! ## This test
//!
//! Simplified reproduction of the incident:
//! - user1 opens SHORT 5 ETH @ $2,000 (margin $1,040 after fees)
//! - user3 opens LONG 5 ETH @ $2,000 (margin $10,000)
//! - user2 places an absurd ask: 1 ETH @ $100,000
//! - Oracle rises to $2,300 → user1 is liquidatable
//! - Liquidation sweeps the absurd ask, then ADL's the remainder at ~-$22,240
//! - user3's margin drops to ~-$87,000 (deeply negative — the bug)
//!
//! ## Expected behavior after fix
//!
//! Bankruptcy price is computed **before** book fills: bp ≈ $2,208. Since equity
//! is negative, the oracle ($2,300) is used as `target_price` — the $100,000
//! ask is skipped. (When equity is positive, bp itself is used as target_price
//! for tighter protection.) All 5 ETH are ADL'd at bp $2,208. user3 receives
//! a modest $1,040 gain (margin → $11,040). No extreme prices.

use {
    crate::register_oracle_prices,
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{self, UserState},
    },
    grug::{Addressable, Coins, QuerierExt, ResultExt, Uint128},
};

/// Reproduces the ADL bankruptcy-price bug with an absurd resting ask.
///
/// | Step | Action                                        | Key numbers                                    |
/// | ---- | --------------------------------------------- | ---------------------------------------------- |
/// | 1    | user3 deposits $10k, places BID 5 ETH @ $2k  | —                                              |
/// | 2    | user1 deposits $1,050, market sells 5 ETH     | fee=$10; margin=$1,040; SHORT 5 @ $2,000       |
/// | 3    | user2 deposits $50k, places ASK 1 ETH @ $100k | absurd resting order                           |
/// | 4    | Oracle → $2,300                               | user1 equity=-$460, MM=$575 → liquidatable     |
/// | 5    | Liquidate user1                               | 1 ETH fills at $100k, 4 ETH ADL'd at ~-$22,240 |
/// | 6    | Verify                                        | user3 margin deeply negative (bug)             |
#[test]
fn adl_bug_absurd_book_price() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    // Oracle = $2,000.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: user3 (ADL counter-party) deposits $10,000, places BID 5 ETH @ $2,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5), // bid (buy)
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 2: user1 (to be liquidated) deposits $1,050, market sells 5 ETH.
    //
    // Fills user3's bid → user1 SHORT 5 @ $2,000, user3 LONG 5 @ $2,000.
    // Taker fee = 5 × $2,000 × 0.1% = $10. user1 margin = $1,040.
    // user3 is maker (no fee), margin stays $10,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(1_050_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5), // ask (sell)
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify positions.
    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(1_040));
    assert_eq!(state.positions[&pair].size, Quantity::new_int(-5));

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(10_000));
    assert_eq!(state.positions[&pair].size, Quantity::new_int(5));

    // -------------------------------------------------------------------------
    // Step 3: user2 deposits $50,000, places absurd ASK: 1 ETH @ $100,000.
    //
    // This order sits on the book at 50× oracle. Under the bug, the liquidation
    // market order sweeps it without any price limit.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(50_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-1), // ask (sell) 1 ETH
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(100_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 4: Oracle → $2,300.
    //
    // user1 (SHORT 5 @ $2,000):
    //   unrealized = (-5) × ($2,300 − $2,000) = −$1,500
    //   equity = $1,040 − $1,500 = −$460
    //   MM = 5 × $2,300 × 5% = $575
    //   −$460 < $575 → liquidatable
    //
    // Close schedule: deficit = $575 − (−$460) = $1,035
    //   close_amount = $1,035 / ($2,300 × 5%) = 9.0 → min(9, 5) = 5
    //   → close entire SHORT position
    // -------------------------------------------------------------------------

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_300);

    // -------------------------------------------------------------------------
    // Step 5: Liquidate user1.
    //
    // BUG BEHAVIOR:
    //   The liquidation market buy has no price limit (target_price = MAX).
    //   It sweeps the book:
    //     - 1 ETH filled at $100,000 (user2's absurd ask)
    //     - 4 ETH unfilled → ADL against user3 (most profitable long)
    //
    //   Post-fill equity:
    //     margin      = $1,040
    //     book_pnl    = 1 × ($2,000 − $100,000) = −$98,000
    //     remaining   = (−4) × ($2,300 − $2,000) = −$1,200
    //     equity      = $1,040 − $98,000 − $1,200 = −$98,160
    //
    //   Buggy bp = $2,300 + (−$98,160) / 4 = −$22,240
    //
    //   user3 forced to sell 4 of 5 ETH at −$22,240:
    //     PnL = 4 × (−$22,240 − $2,000) = −$96,960
    //     margin after = $10,000 − $96,960 = −$86,960
    //
    // CORRECT BEHAVIOR (after fix):
    //   equity = −$460 (negative), so target_price = oracle ($2,300)
    //   bp = $2,300 + (−$460) / 5 = $2,208 (computed before book fills)
    //   Absurd ask at $100,000 >> oracle → skipped
    //   All 5 ETH unfilled → ADL'd at bp $2,208
    //   user3 PnL = 5 × ($2,208 − $2,000) = +$1,040
    //   user3 margin = $10,000 + $1,040 = $11,040
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: accounts.user1.address(),
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 6: Verify the fix works.
    //
    // With the fix, the bankruptcy price is computed BEFORE book fills
    // (bp ≈ $2,208). Since equity is negative, the oracle price ($2,300)
    // is used as target_price for book matching. The absurd $100,000 ask
    // is far above oracle and is skipped. All 5 ETH go to ADL at bp.
    // -------------------------------------------------------------------------

    // user1 (liquidated): should have no positions and ~$0 margin.
    let user1_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert!(
        user1_state.is_none() || user1_state.as_ref().unwrap().positions.is_empty(),
        "user1 should have no positions after liquidation"
    );

    // user3 (ADL counter-party):
    //
    // Before the fix: margin was deeply negative (~-$87k) because they
    // were forced to sell at an extreme negative ADL price (-$22,240).
    //
    // After the fix: all 5 ETH ADL'd at bp ≈ $2,208.
    //   PnL = 5 × ($2,208 − $2,000) = +$1,040
    //   margin = $10,000 + $1,040 = $11,040
    let user3_state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();

    // Before the fix, this was:
    //   assert!(user3_state.margin.is_negative(), ...);
    // The bug caused ADL at -$22,240, giving user3 a PnL of -$96,960
    // and margin of -$86,960. With the fix, bp ≈ $2,208 and user3
    // receives a modest gain instead.
    assert!(
        user3_state.margin.is_positive(),
        "user3 margin should be positive after fix, got {:?}",
        user3_state.margin,
    );
    assert!(
        user3_state.positions.is_empty(),
        "user3's entire LONG should be closed via ADL"
    );

    // user2 (absurd order placer):
    //
    // Before the fix: the absurd ask at $100,000 was filled, giving user2
    // a SHORT 1 ETH @ $100,000.
    //
    // After the fix: the bankruptcy price ($2,208) is used as target_price,
    // so the $100,000 ask is never matched. user2 should have no position.
    let user2_state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user2.address(),
        })
        .should_succeed()
        .unwrap();

    // Before the fix, these asserted the absurd fill happened:
    //   assert!(user2_state.positions.contains_key(&pair), ...);
    //   assert_eq!(user2_state.positions[&pair].size, Quantity::new_int(-1), ...);
    //   assert_eq!(user2_state.positions[&pair].entry_price, UsdPrice::new_int(100_000), ...);
    assert!(
        !user2_state.positions.contains_key(&pair),
        "user2's absurd ask at $100,000 should NOT have been filled"
    );
}
