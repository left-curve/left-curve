//! Regression tests for pre-fill margin check gaps.
//!
//! The pre-fill margin check (`check_margin`) computes equity and projected
//! initial margin at oracle prices, ignoring that the actual fill price may
//! differ drastically. This allows orders to push accounts into deeply negative
//! equity through realized PnL at bad fill prices.
//!
//! **Group A** (tests 1–4): Reduce-only orders skip the margin check entirely.
//! **Group B** (tests 5–6): Non-reduce-only partial closes pass the check but
//! still push margin negative because the check doesn't account for PnL
//! realization at the execution price.

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

// =============================================================================
// Group A: Reduce-only orders (margin check skipped entirely)
// =============================================================================

/// Reduce-only market sell closes a long at a catastrophically low price.
///
/// | Step | Action                                               | Key numbers                                               |
/// |------|------------------------------------------------------|-----------------------------------------------------------|
/// | 1    | User2 deposits $100k; PostOnly sell 10 ETH @ $2,000  | provides opening-side liquidity                           |
/// | 2    | User1 deposits $10k; market buys 10 ETH              | fee=$20; margin=$9,980; long 10 @ $2,000                  |
/// | 3    | User3 deposits $100k; PostOnly buy 10 ETH @ $200     | bad-price bid                                             |
/// | 4    | User1 reduce-only market sell 10 ETH (90% slippage)  | target=$200; fills @ $200; PnL=−$18,000; fee=$2           |
/// | 5    | Assert                                               | margin=−$8,022; position closed; account deeply negative  |
#[test]
fn reduce_only_market_sell_pushes_margin_negative() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // Step 1: User2 provides opening liquidity (PostOnly sell 10 ETH @ $2,000).

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 2: User1 deposits $10,000 and market buys 10 ETH.
    // Fee = 10 × $2,000 × 0.1% = $20. Margin = $9,980.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(1),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(9_980));
    assert_eq!(
        state.positions.get(&pair).unwrap().size,
        Quantity::new_int(10)
    );

    // Step 3: User3 places bad-price bid (PostOnly buy 10 ETH @ $200).

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(200),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 4: User1 reduce-only market sells 10 ETH with 90% slippage.
    // Target = $2,000 × 0.1 = $200. Now rejected by the pre-fill margin check.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(90),
                },
                reduce_only: true,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient margin");

    // Old behavior (before fix): order succeeded and pushed margin negative.
    // let state: UserState = suite
    //     .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
    //         user: accounts.user1.address(),
    //     })
    //     .should_succeed()
    //     .unwrap();
    // assert!(state.positions.is_empty());
    // assert_eq!(state.margin, UsdValue::new_int(-8_022));
}

/// Reduce-only market buy closes a short at a catastrophically high price.
///
/// | Step | Action                                               | Key numbers                                               |
/// |------|------------------------------------------------------|-----------------------------------------------------------|
/// | 1    | User2 deposits $100k; PostOnly buy 10 ETH @ $2,000   | provides opening-side liquidity                           |
/// | 2    | User1 deposits $10k; market sells 10 ETH             | fee=$20; margin=$9,980; short 10 @ $2,000                 |
/// | 3    | User3 deposits $100k; PostOnly sell 10 ETH @ $3,800  | bad-price ask                                             |
/// | 4    | User1 reduce-only market buy 10 ETH (90% slippage)   | target=$3,800; fills @ $3,800; PnL=−$18,000; fee=$38     |
/// | 5    | Assert                                               | margin=−$8,058; position closed; account deeply negative  |
#[test]
fn reduce_only_market_buy_pushes_margin_negative() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // Step 1: User2 provides opening liquidity (PostOnly buy 10 ETH @ $2,000).

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 2: User1 deposits $10,000 and market sells 10 ETH.
    // Fee = 10 × $2,000 × 0.1% = $20. Margin = $9,980.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(1),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(9_980));
    assert_eq!(
        state.positions.get(&pair).unwrap().size,
        Quantity::new_int(-10)
    );

    // Step 3: User3 places bad-price ask (PostOnly sell 10 ETH @ $3,800).

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(3_800),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 4: User1 reduce-only market buys 10 ETH with 90% slippage.
    // Target = $2,000 × 1.9 = $3,800. Now rejected by the pre-fill margin check.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(90),
                },
                reduce_only: true,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient margin");

    // Old behavior (before fix): order succeeded and pushed margin negative.
    // let state: UserState = suite
    //     .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
    //         user: accounts.user1.address(),
    //     })
    //     .should_succeed()
    //     .unwrap();
    // assert!(state.positions.is_empty());
    // assert_eq!(state.margin, UsdValue::new_int(-8_058));
}

/// Reduce-only GTC limit sell rests at a terrible price, then a taker fills it.
///
/// | Step | Action                                               | Key numbers                                               |
/// |------|------------------------------------------------------|-----------------------------------------------------------|
/// | 1    | User2 deposits $100k; PostOnly sell 10 ETH @ $2,000  | provides opening-side liquidity                           |
/// | 2    | User1 deposits $10k; market buys 10 ETH              | fee=$20; margin=$9,980; long 10 @ $2,000                  |
/// | 3    | User1 places reduce-only GTC sell 10 ETH @ $200      | rests as ask; margin check skipped                        |
/// | 4    | User3 deposits $100k; market buys 10 ETH             | fills against User1's ask @ $200; User1 PnL=−$18,000     |
/// | 5    | Assert                                               | margin=−$8,020; position closed; account deeply negative  |
#[test]
fn reduce_only_limit_sell_pushes_margin_negative() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // Step 1: User2 provides opening liquidity (PostOnly sell 10 ETH @ $2,000).

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 2: User1 deposits $10,000 and market buys 10 ETH.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(1),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(9_980));

    // Step 3: User1 places reduce-only GTC sell 10 ETH @ $200.
    // Margin check skipped because reduce_only = true. Order rests as ask.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(200),
                    time_in_force: perps::TimeInForce::GoodTilCanceled,
                },
                reduce_only: true,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient margin");

    // Old behavior (before fix): order rested, then step 4 filled and pushed margin negative.
    // Step 4 (User3 market buys) and step 5 assertions are now unreachable.
    // let state: UserState = suite
    //     .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
    //         user: accounts.user1.address(),
    //     })
    //     .should_succeed()
    //     .unwrap();
    // assert!(state.positions.is_empty());
    // assert_eq!(state.margin, UsdValue::new_int(-8_020));
}

/// Reduce-only GTC limit buy rests at a terrible price, then a taker fills it.
///
/// | Step | Action                                               | Key numbers                                               |
/// |------|------------------------------------------------------|-----------------------------------------------------------|
/// | 1    | User2 deposits $100k; PostOnly buy 10 ETH @ $2,000   | provides opening-side liquidity                           |
/// | 2    | User1 deposits $10k; market sells 10 ETH             | fee=$20; margin=$9,980; short 10 @ $2,000                 |
/// | 3    | User1 places reduce-only GTC buy 10 ETH @ $3,800     | now rejected by margin check in store_limit_order         |
#[test]
fn reduce_only_limit_buy_pushes_margin_negative() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // Step 1: User2 provides opening liquidity (PostOnly buy 10 ETH @ $2,000).

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 2: User1 deposits $10,000 and market sells 10 ETH.
    // Fee = 10 × $2,000 × 0.1% = $20. Margin = $9,980.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(1),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(9_980));
    assert_eq!(
        state.positions.get(&pair).unwrap().size,
        Quantity::new_int(-10)
    );

    // Step 3: User1 places reduce-only GTC buy 10 ETH @ $3,800.
    // Margin check skipped because reduce_only = true. Order rests as bid.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(3_800),
                    time_in_force: perps::TimeInForce::GoodTilCanceled,
                },
                reduce_only: true,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient margin");

    // Old behavior (before fix): order rested, then step 4 filled and pushed margin negative.
    // Step 4 (User3 market sells) and step 5 assertions are now unreachable.
    // let state: UserState = suite
    //     .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
    //         user: accounts.user1.address(),
    //     })
    //     .should_succeed()
    //     .unwrap();
    // assert!(state.positions.is_empty());
    // assert_eq!(state.margin, UsdValue::new_int(-8_020));
}

// =============================================================================
// Group B: Non-reduce-only partial close (margin check passes but insufficient)
// =============================================================================

/// Non-reduce-only sell partially closes a long at a bad price. The pre-fill
/// check now catches the catastrophic PnL at the target price and rejects.
///
/// | Step | Action                                               | Key numbers                                               |
/// |------|------------------------------------------------------|-----------------------------------------------------------|
/// | 1    | User2 deposits $100k; PostOnly sell 10 ETH @ $2,000  | provides opening-side liquidity                           |
/// | 2    | User1 deposits $3k; market buys 10 ETH               | fee=$20; margin=$2,980; long 10 @ $2,000                  |
/// | 3    | User3 deposits $100k; PostOnly buy 5 ETH @ $200      | bad-price bid for partial close                           |
/// | 4    | User1 market sells 5 ETH (90% slippage, NOT reduce)  | check passes; fills @ $200; PnL=−$9,000; fee=$1          |
/// | 5    | Assert                                               | margin=−$6,021; long 5 remains; equity ≪ MM=$500         |
#[test]
fn partial_close_sell_at_bad_price_pushes_margin_negative() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // Step 1: User2 provides opening liquidity (PostOnly sell 10 ETH @ $2,000).

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 2: User1 deposits $3,000 (minimum viable) and market buys 10 ETH.
    // Margin check: equity=$3,000 ≥ IM($2,000) + fee($20) = $2,020. Passes.
    // Fee = 10 × $2,000 × 0.1% = $20. Margin = $2,980.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(3_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(1),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(2_980));

    // Step 3: User3 places bad-price bid (PostOnly buy 5 ETH @ $200).

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(200),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 4: User1 market sells 5 ETH (NOT reduce-only) with 90% slippage.
    // Pre-fill check: equity=$2,980, projSize=5, projIM=5×$2k×10%=$1,000,
    //   fee=5×$2k×0.1%=$10, required=$1,010. $2,980 ≥ $1,010 → passes.
    // Fills at $200: PnL = 5 × ($200 − $2,000) = −$9,000. Fee = $1.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(90),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient margin");

    // Old behavior (before fix): order succeeded and pushed margin negative.
    // let state: UserState = suite
    //     .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
    //         user: accounts.user1.address(),
    //     })
    //     .should_succeed()
    //     .unwrap();
    // assert_eq!(state.margin, UsdValue::new_int(-6_021));
    // let pos = state.positions.get(&pair).unwrap();
    // assert_eq!(pos.size, Quantity::new_int(5));
    // assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
    // // Equity = −$6,021. MM = $500. Account immediately liquidatable.
}

/// Non-reduce-only buy partially closes a short at a bad price. The pre-fill
/// check now catches the catastrophic PnL at the target price and rejects.
///
/// | Step | Action                                               | Key numbers                                               |
/// |------|------------------------------------------------------|-----------------------------------------------------------|
/// | 1    | User2 deposits $100k; PostOnly buy 10 ETH @ $2,000   | provides opening-side liquidity                           |
/// | 2    | User1 deposits $3k; market sells 10 ETH              | fee=$20; margin=$2,980; short 10 @ $2,000                 |
/// | 3    | User3 deposits $100k; PostOnly sell 5 ETH @ $3,800   | bad-price ask for partial close                           |
/// | 4    | User1 market buys 5 ETH (90% slippage, NOT reduce)   | check passes; fills @ $3,800; PnL=−$9,000; fee=$19       |
/// | 5    | Assert                                               | margin=−$6,039; short 5 remains; equity ≪ MM=$500        |
#[test]
fn partial_close_buy_at_bad_price_pushes_margin_negative() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
    let pair = pair_id();

    // Step 1: User2 provides opening liquidity (PostOnly buy 10 ETH @ $2,000).

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 2: User1 deposits $3,000 and market sells 10 ETH.
    // Fee = 10 × $2,000 × 0.1% = $20. Margin = $2,980.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(3_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(1),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(2_980));
    assert_eq!(
        state.positions.get(&pair).unwrap().size,
        Quantity::new_int(-10)
    );

    // Step 3: User3 places bad-price ask (PostOnly sell 5 ETH @ $3,800).

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(3_800),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Step 4: User1 market buys 5 ETH (NOT reduce-only) with 90% slippage.
    // Now rejected: the pre-fill check simulates the fill at target=$3,800 and
    // sees the resulting equity would be deeply negative.

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(90),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("insufficient margin");

    // Old behavior (before fix): order succeeded and pushed margin negative.
    // let state: UserState = suite
    //     .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
    //         user: accounts.user1.address(),
    //     })
    //     .should_succeed()
    //     .unwrap();
    // assert_eq!(state.margin, UsdValue::new_int(-6_039));
    // let pos = state.positions.get(&pair).unwrap();
    // assert_eq!(pos.size, Quantity::new_int(-5));
    // assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
    // // Equity = −$6,039. MM = $500. Account immediately liquidatable.
}
