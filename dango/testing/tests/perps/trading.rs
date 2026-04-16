use {
    crate::{default_pair_param, default_param, register_oracle_prices},
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{self, LiquidityDepthResponse, PairParam, Param, UserState},
    },
    grug::{Addressable, Coins, QuerierExt, ResultExt, Uint128, btree_map, btree_set},
    std::collections::BTreeMap,
};

/// Covers: deposit, market full fill, withdraw success, withdraw fail.
///
/// | Step | Action                              | Key numbers                                                   | Assert                                                       |
/// | ---- | ----------------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------ |
/// | 1    | Trader starts with margin = $10,000 | —                                                             | `user_state.margin = $10,000`                                |
/// | 2    | Maker A places ask: 10 ETH @ $2,000 | resting on book                                               | ask exists in ASKS                                           |
/// | 3    | Trader market buys 10 ETH           | fee = 10 × $2,000 × 0.1% = $20                                | position: 10 ETH long @ $2,000; margin = $9,980; ask removed |
/// | 4    | Trader withdraws $7,000             | equity=$9,980, used_IM=10×$2,000×10%=$2,000, available=$7,980 | succeeds; margin = $2,980                                    |
/// | 5    | Trader withdraws $2,000             | available = $2,980 - $2,000 = $980                            | fails: "exceeds available margin"                            |
#[test]
fn trading_lifecycle() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    // Register oracle prices: ETH = $2,000, USDC = $1.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: Trader (user1) deposits $10,000 USDC.
    // USDC has 6 decimals, so $10,000 = 10_000_000_000 base units.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Verify trader's margin = $10,000.
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(10_000));

    // -------------------------------------------------------------------------
    // Step 2: Maker (user2) deposits $10,000 USDC and places ask: 10 ETH @ $2,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10), // sell / ask
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

    // Verify ask exists on the book.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();

    assert_eq!(orders.len(), 1, "maker should have 1 ask");

    // -------------------------------------------------------------------------
    // Step 3: Trader market buys 10 ETH.
    // Fee = 10 * $2,000 * 0.1% = $20.  Margin after = $10,000 - $20 = $9,980.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10), // buy
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify position and margin.
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let pos = state
        .positions
        .get(&pair)
        .expect("should have ETH position");

    assert_eq!(pos.size, Quantity::new_int(10), "should be 10 ETH long");
    assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
    assert_eq!(
        state.margin,
        UsdValue::new_int(9_980),
        "margin should be $9,980 after $20 fee"
    );

    // Maker's ask should be removed.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();

    assert!(
        orders.is_empty(),
        "maker ask should be fully filled and removed"
    );

    // -------------------------------------------------------------------------
    // Step 4: Trader withdraws $7,000 (should succeed).
    // equity = $9,980, used IM = 10 * $2,000 * 10% = $2,000, available = $7,980.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Withdraw {
                amount: UsdValue::new_int(7_000),
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify margin = $9,980 - $7,000 = $2,980.
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(2_980));

    // -------------------------------------------------------------------------
    // Step 5: Trader withdraws $2,000 (should fail).
    // available = $2,980 - IM($2,000) = $980 < $2,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Withdraw {
                amount: UsdValue::new_int(2_000),
            }),
            Coins::new(),
        )
        .should_fail_with_error("exceeds available margin");
}

/// Covers: limit partial fill (rest persists to book), cancel order.
///
/// | Step | Action                              | Key numbers                        | Assert                                                                                                                                                 |
/// | ---- | ----------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
/// | 1    | Trader margin = $10,000             | —                                  | —                                                                                                                                                      |
/// | 2    | Maker A places ask: 5 ETH @ $2,000  | —                                  | —                                                                                                                                                      |
/// | 3    | Trader limit buys 10 ETH @ $2,000   | 5 filled vs maker, 5 rests as bid  | position: 5 ETH long @ $2,000; fee = $10; margin = $9,990; reserved_margin = 5x$2,000x10% = $1,000 (for resting 5); open_order_count = 1; bid on book  |
/// | 4    | Trader cancels the resting order    | —                                  | reserved_margin = $0; open_order_count = 0; bid removed from book; position unchanged (still 5 ETH)                                                    |
#[test]
fn limit_order_partial_fill_and_cancel() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    // Register oracle prices: ETH = $2,000, USDC = $1.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: Trader (user1) deposits $10,000 USDC.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 2: Maker (user2) deposits $10,000 USDC and places ask: 5 ETH @ $2,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5), // sell / ask 5 ETH
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

    // -------------------------------------------------------------------------
    // Step 3: Trader limit buys 10 ETH @ $2,000 (NOT post_only).
    // 5 fill against maker's ask, 5 rest as a bid on the book.
    // Fee = 5 * $2,000 * 0.1% = $10 (taker fee on the filled portion).
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10), // buy 10 ETH
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::GoodTilCanceled,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify position: 5 ETH long @ $2,000, margin = $9,990.
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let pos = state
        .positions
        .get(&pair)
        .expect("should have ETH position");

    assert_eq!(pos.size, Quantity::new_int(5), "should be 5 ETH long");
    assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
    assert_eq!(
        state.margin,
        UsdValue::new_int(9_990),
        "margin should be $9,990 after $10 fee"
    );

    // Verify reserved_margin = $1,000 for the resting 5 ETH.
    assert_eq!(
        state.reserved_margin,
        UsdValue::new_int(1_000),
        "reserved_margin should be $1,000 for 5 resting ETH"
    );

    // Verify open_order_count = 1.
    assert_eq!(state.open_order_count, 1, "should have 1 open order");

    // Verify bid exists on the book.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert_eq!(orders.len(), 1, "trader should have 1 resting bid");

    // -------------------------------------------------------------------------
    // Step 4: Trader cancels the resting order.
    // -------------------------------------------------------------------------

    let order_id = *orders.keys().next().unwrap();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::One(order_id),
            )),
            Coins::new(),
        )
        .should_succeed();

    // Verify reserved_margin = $0 and open_order_count = 0.
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(
        state.reserved_margin,
        UsdValue::ZERO,
        "reserved_margin should be $0 after cancel"
    );
    assert_eq!(state.open_order_count, 0, "should have 0 open orders");

    // Verify bid removed from book.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert!(orders.is_empty(), "orders should be empty after cancel");

    // Verify position unchanged: still 5 ETH long @ $2,000.
    let pos = state
        .positions
        .get(&pair)
        .expect("should still have ETH position");

    assert_eq!(pos.size, Quantity::new_int(5), "should still be 5 ETH long");
    assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
}

/// Verify that liquidity depth bookkeeping tracks resting orders correctly
/// across four code paths: order placement, self-trade prevention (EXPIRE_MAKER),
/// fill, and cancel-all.
#[test]
fn liquidity_depth_tracking() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // Configure pair with a $100 bucket size for depth tracking.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: default_param(),
                pair_params: btree_map! {
                    pair.clone() => PairParam {
                        bucket_sizes: btree_set! { UsdPrice::new_int(100) },
                        ..default_pair_param()
                    },
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Deposit margin for both users.
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
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    let query_depth = |suite: &dango_testing::TestSuite<_>| -> LiquidityDepthResponse {
        suite
            .query_wasm_smart(contracts.perps, perps::QueryLiquidityDepthRequest {
                pair_id: pair.clone(),
                bucket_size: UsdPrice::new_int(100),
                limit: None,
            })
            .should_succeed()
    };

    // -------------------------------------------------------------------------
    // Step 1: Initial state — no depth.
    // -------------------------------------------------------------------------

    let depth = query_depth(&suite);
    assert!(depth.bids.is_empty(), "bids should be empty initially");
    assert!(depth.asks.is_empty(), "asks should be empty initially");

    // -------------------------------------------------------------------------
    // Step 2: user1 places ask: sell 3 ETH @ $2,000 (post_only).
    // This order will be STP-cancelled when user1 later submits a buy.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-3),
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

    let depth = query_depth(&suite);

    assert!(depth.bids.is_empty(), "bids should still be empty");
    assert_eq!(depth.asks.len(), 1, "asks should have 1 bucket");

    let ask_bucket = depth.asks.get(&UsdPrice::new_int(2_000)).unwrap();

    assert_eq!(ask_bucket.size, Quantity::new_int(3), "ask size = 3");
    assert_eq!(
        ask_bucket.notional,
        UsdValue::new_int(6_000),
        "ask notional = $6,000"
    );

    // -------------------------------------------------------------------------
    // Step 3: user2 places ask: sell 5 ETH @ $2,000 (post_only).
    // Ask depth accumulates: 3 + 5 = 8 ETH.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
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

    let depth = query_depth(&suite);

    assert!(depth.bids.is_empty(), "bids should still be empty");
    assert_eq!(depth.asks.len(), 1, "asks should have 1 bucket");

    let ask_bucket = depth.asks.get(&UsdPrice::new_int(2_000)).unwrap();

    assert_eq!(ask_bucket.size, Quantity::new_int(8), "ask size = 8");
    assert_eq!(
        ask_bucket.notional,
        UsdValue::new_int(16_000),
        "ask notional = $16,000"
    );

    // -------------------------------------------------------------------------
    // Step 4: user1 limit buys 10 ETH @ $2,000 (NOT post_only).
    // The matching engine walks the ask book:
    //   - user1's own 3 ETH ask → EXPIRE_MAKER cancels it (ask depth −3).
    //     Taker does NOT consume these; remaining taker size stays 10.
    //   - user2's 5 ETH ask → fills 5 (ask depth −5). Remaining = 5.
    //   - No more asks → 5 ETH rests as bid (bid depth +5).
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::GoodTilCanceled,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    let depth = query_depth(&suite);

    assert!(
        depth.asks.is_empty(),
        "asks should be empty after STP + fill"
    );
    assert_eq!(depth.bids.len(), 1, "bids should have 1 bucket");

    let bid_bucket = depth.bids.get(&UsdPrice::new_int(2_000)).unwrap();

    assert_eq!(bid_bucket.size, Quantity::new_int(5), "bid size = 5");
    assert_eq!(
        bid_bucket.notional,
        UsdValue::new_int(10_000),
        "bid notional = $10,000"
    );

    // -------------------------------------------------------------------------
    // Step 5: Cancel all user1's orders → bid depth drops to zero.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::All,
            )),
            Coins::new(),
        )
        .should_succeed();

    let depth = query_depth(&suite);

    assert!(
        depth.bids.is_empty(),
        "bids should be empty after cancel-all"
    );
    assert!(
        depth.asks.is_empty(),
        "asks should be empty after cancel-all"
    );
}

/// Covers: protocol treasury fees accumulate across multiple fills.
///
/// This is a regression test for a bug where `settle_pnls` used
/// `maker_states.entry(protocol_treasury).or_default()` — which created a
/// blank `UserState` every call, losing previously accumulated fees.
///
/// After the fix, protocol fees are stored in `State::treasury` (analogous
/// to `State::insurance_fund`) and accumulate correctly.
///
/// | Step | Action                                 | Assert                                                  |
/// | ---- | -------------------------------------- | ------------------------------------------------------- |
/// | 1    | Configure: protocol_fee_rate = 20%     | —                                                       |
/// | 2    | Maker places ask 10 ETH @ $2,000       | —                                                       |
/// | 3    | Taker market buys 10 ETH               | fee=$20, protocol=$4 → State.treasury == $4             |
/// | 4    | Maker places another ask 10 ETH @ $2k  | —                                                       |
/// | 5    | Taker market buys 10 ETH               | another $4 → State.treasury == $8 (accumulated!)        |
#[test]
fn protocol_fee_accumulates_across_fills() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // Configure: set protocol_fee_rate = 20%.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: Param {
                    protocol_fee_rate: Dimensionless::new_percent(20),
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair.clone() => default_pair_param(),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Deposit for maker (user2) and taker (user1).
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
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    // --- Fill 1: Maker places ask 10 ETH @ $2,000, taker market buys ---

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

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // fee = 10 * $2,000 * 0.1% = $20
    // protocol_fee = $20 * 20% = $4
    let global_state: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_state.treasury,
        UsdValue::new_int(4),
        "treasury should be $4 after first fill"
    );

    // --- Fill 2: Maker places another ask 10 ETH @ $2,000, taker buys ---

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

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // another $4 → total should be $8 (accumulated, not overwritten!)
    let global_state: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_state.treasury,
        UsdValue::new_int(8),
        "treasury should be $8 after second fill (accumulated, not overwritten)"
    );
}

/// Negative maker fee (rebate): maker is paid on every fill.
///
/// | Step | Action                                          | Key numbers                                                        | Assert                                   |
/// | ---- | ----------------------------------------------- | ------------------------------------------------------------------ | ---------------------------------------- |
/// | 1    | Configure: taker=3 bps, maker=-1 bps, proto=20%| —                                                                  | config accepted                          |
/// | 2    | Deposit $100,000 each for taker and maker       | —                                                                  | margins = $100,000                       |
/// | 3    | Maker places post_only ask: 50 ETH @ $2,000    | resting on book                                                    | ask exists                               |
/// | 4    | Taker market buys 50 ETH                        | notional=$100k, taker fee=$30, maker fee=-$10                      | taker margin=$99,970; maker margin=$100,010 |
/// | 5    | Check treasury                                  | proto: taker $6 + maker -$2 = $4                                   | treasury=$4                              |
#[test]
fn negative_maker_fee_rebate_lifecycle() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: Configure taker = 3 bps, maker = -1 bps, protocol_fee = 20%.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: Param {
                    taker_fee_rates: perps::RateSchedule {
                        base: Dimensionless::new_raw(300), // 3 bps
                        ..Default::default()
                    },
                    maker_fee_rates: perps::RateSchedule {
                        base: Dimensionless::new_raw(-100), // -1 bps (rebate)
                        ..Default::default()
                    },
                    protocol_fee_rate: Dimensionless::new_percent(20),
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair.clone() => default_pair_param(),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 2: Deposit $100,000 each for maker (user2) and taker (user1).
    // -------------------------------------------------------------------------

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
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 3: Maker (user2) places post_only ask: sell 50 ETH @ $2,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-50),
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

    // -------------------------------------------------------------------------
    // Step 4: Taker (user1) market buys 50 ETH.
    //
    // Notional = 50 × $2,000 = $100,000
    // Taker fee = $100,000 × 3 bps = $30
    // Maker fee = $100,000 × (-1 bps) = -$10 (rebate)
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(50),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 4 assertions: check margins.
    // -------------------------------------------------------------------------

    // Taker: $100,000 - $30 fee = $99,970.
    let taker_state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(
        taker_state.margin,
        UsdValue::new_int(99_970),
        "taker margin should be $99,970 after paying $30 fee"
    );

    // Maker: $100,000 + $10 rebate = $100,010.
    let maker_state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user2.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(
        maker_state.margin,
        UsdValue::new_int(100_010),
        "maker margin should be $100,010 after receiving $10 rebate"
    );

    // -------------------------------------------------------------------------
    // Step 5: Check treasury.
    //
    // Protocol fee from taker: $30 × 20% = $6
    // Protocol fee from maker: -$10 × 20% = -$2
    // Net treasury = $4
    // -------------------------------------------------------------------------

    let global_state: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_state.treasury,
        UsdValue::new_int(4),
        "treasury should be $4 (taker $6 + maker -$2)"
    );
}

/// IOC limit order: partial fill with unfilled remainder cancelled (not stored).
///
/// | Step | Action                                        | Assert                                          |
/// |------|-----------------------------------------------|-------------------------------------------------|
/// | 1    | Both users deposit $10,000 USDC               | margin established                              |
/// | 2    | Maker (user2) places post-only ask: 5 ETH     | ask on book                                     |
/// | 3    | Taker (user1) IOC limit buy 10 ETH @ $2,000   | 5 fill, 5 cancelled                             |
/// | 4    | Verify taker state                            | position=5, open_order_count=0, reserved=$0     |
#[test]
fn ioc_limit_order_partial_fill() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: Both users deposit $10,000 USDC.
    // -------------------------------------------------------------------------

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
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 2: Maker (user2) places post-only ask: sell 5 ETH @ $2,000.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
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

    // -------------------------------------------------------------------------
    // Step 3: Taker (user1) IOC limit buy 10 ETH @ $2,000.
    // 5 fill against maker, 5 unfilled → cancelled (IOC).
    // Fee = 5 × $2,000 × 0.1% = $10.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::ImmediateOrCancel,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 4: Verify taker state.
    // -------------------------------------------------------------------------

    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    let pos = state
        .positions
        .get(&pair)
        .expect("should have ETH position");

    assert_eq!(pos.size, Quantity::new_int(5), "should be 5 ETH long");
    assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
    assert_eq!(
        state.margin,
        UsdValue::new_int(9_990),
        "margin should be $9,990 after $10 taker fee"
    );

    // IOC: no resting order.
    assert_eq!(
        state.reserved_margin,
        UsdValue::ZERO,
        "reserved_margin should be $0 (IOC cancelled unfilled)"
    );
    assert_eq!(state.open_order_count, 0, "should have 0 open orders");

    // Verify no resting orders on book.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert!(orders.is_empty(), "IOC taker should have no resting orders");
}

/// IOC limit order with zero fills errors out.
///
/// | Step | Action                                          | Assert                         |
/// |------|-------------------------------------------------|--------------------------------|
/// | 1    | Taker deposits $10,000 USDC                     | margin established             |
/// | 2    | Taker IOC limit buy 10 ETH @ $1,900 (empty book)| error: no liquidity            |
#[test]
fn ioc_limit_order_no_fill_rejected() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // Step 1: Deposit.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Step 2: IOC limit buy against empty book → should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_900),
                    time_in_force: perps::TimeInForce::ImmediateOrCancel,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("no liquidity at acceptable price");
}

// ============================ price banding ===================================
//
// All band tests use a 10% band (`max_limit_price_deviation = 0.1`) with
// oracle = $2,000. Allowed range is [$1,800, $2,200].

/// Set up a test suite with oracle = $2,000, band = 10%, and user1 funded
/// with $10,000 margin.
macro_rules! setup_band_suite {
    () => {{
        let (mut suite, mut accounts, _, contracts, _) =
            setup_test_naive(TestOption::default());
        register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);
        let pair = pair_id();

        suite
            .execute(
                &mut accounts.owner,
                contracts.perps,
                &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                    param: default_param(),
                    pair_params: btree_map! {
                        pair.clone() => PairParam {
                            max_limit_price_deviation: Dimensionless::new_permille(100), // 10%
                            ..default_pair_param()
                        },
                    },
                }),
                Coins::new(),
            )
            .should_succeed();

        suite
            .execute(
                &mut accounts.user1,
                contracts.perps,
                &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
                Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
            )
            .should_succeed();

        (suite, accounts, contracts, pair)
    }};
}

/// Send a limit order from user1 at the given price and time-in-force.
macro_rules! submit_limit {
    ($suite:expr, $accounts:expr, $contracts:expr, $pair:expr, $size:expr, $price:expr, $tif:expr) => {
        $suite.execute(
            &mut $accounts.user1,
            $contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: $pair.clone(),
                size: Quantity::new_int($size),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int($price),
                    time_in_force: $tif,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
    };
}

/// GTC limit buy exactly at the upper band bound is accepted.
#[test]
fn banding_gtc_at_upper_bound_accepted() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    // oracle * (1 + 10%) = $2,200.
    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        1,
        2_200,
        perps::TimeInForce::GoodTilCanceled
    )
    .should_succeed();
}

/// GTC limit buy just above the upper bound is rejected.
#[test]
fn banding_gtc_just_above_upper_bound_rejected() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        1,
        2_201,
        perps::TimeInForce::GoodTilCanceled
    )
    .should_fail_with_error("deviates too far");
}

/// GTC limit sell below the lower bound is rejected.
#[test]
fn banding_gtc_below_lower_bound_rejected() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        -1,
        1_799,
        perps::TimeInForce::GoodTilCanceled
    )
    .should_fail_with_error("deviates too far");
}

/// IOC limit with out-of-band price is rejected before any fill is attempted.
#[test]
fn banding_ioc_out_of_band_rejected() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        1,
        3_000,
        perps::TimeInForce::ImmediateOrCancel
    )
    .should_fail_with_error("deviates too far");
}

/// PostOnly with out-of-band price is rejected at Step 0, never reaching the
/// crossing check inside `store_post_only_limit_order`.
#[test]
fn banding_post_only_out_of_band_rejected() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        -1,
        1_000,
        perps::TimeInForce::PostOnly
    )
    .should_fail_with_error("deviates too far");
}

/// Market orders use `max_slippage`, not `limit_price`, so the band does not
/// apply. A market order with wide slippage against an empty book fails only
/// for lack of liquidity, not for banding reasons.
#[test]
fn banding_does_not_affect_market_orders() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(1),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_permille(500), // 50%
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_fail_with_error("no liquidity at acceptable price");
}

/// Attack trap-setting regression. An attacker attempting to rest a PostOnly
/// bid far below oracle — the precondition for the bad-debt-minting self-match
/// attack — is rejected at submission. Without banding, this order would rest
/// on the book and serve as the bad-price maker in the attack.
#[test]
fn banding_blocks_pathological_post_only_trap() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    // Oracle = $2,000, band = 10%. A PostOnly bid at $200 (90% below
    // oracle) would be the trap leg of a bad-debt-minting attack.
    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        1,
        200,
        perps::TimeInForce::PostOnly
    )
    .should_fail_with_error("deviates too far");
}

/// Drift variant: a maker order placed within the band at T1 falls outside
/// the band at T2 after the oracle moves. The match-time re-check cancels
/// the stale order when another user tries to match against it.
///
/// Without this check, an attacker could place a legal PostOnly at T1 and
/// wait for natural or nudged oracle drift to make the resting price
/// pathological, reproducing the bad-debt attack despite submission-time
/// banding.
///
/// Test shape: the walk encounters the stale maker first (near-end drift),
/// cancels it, and then fills against an in-band maker deeper in the book.
/// This verifies `continue` semantics (walk past the cancelled maker,
/// rather than `break`-ing and leaving in-band liquidity unreached).
#[test]
fn banding_drift_maker_cancelled_at_match_time() {
    let (mut suite, mut accounts, contracts, pair) = setup_band_suite!();

    // Oracle at T1 = $2,000, band = 10% → allowed range [$1,800, $2,200].
    //
    // user1 (to-be-stale) places PostOnly ask at $2,199 (in-band at T1;
    // will be below the lower bound after oracle moves up).
    submit_limit!(
        suite,
        accounts,
        contracts,
        pair,
        -1,
        2_199,
        perps::TimeInForce::PostOnly
    )
    .should_succeed();

    // Oracle moves to $2,500 → new allowed range [$2,250, $2,750].
    //   - user1's $2,199 ask: now below the lower bound ($2,199 < $2,250).
    //     OUT of band.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_500);

    // user3 deposits and (at T2, in the new band) places a PostOnly ask
    // at $2,400 — this one stays in-band.
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
                size: Quantity::new_int(-1),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_400),
                    time_in_force: perps::TimeInForce::PostOnly,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // user2 market-buys size 2 with wide slippage. Walks asks ascending:
    //   1. user1 at $2,199 (out-of-band) → drift-cancel, continue.
    //   2. user3 at $2,400 (in-band) → fills 1. remaining 1.
    //   3. No more asks → partial fill; tx succeeds.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(2),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // user1's stale ask was cancelled by the match-time check.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert!(
        orders.is_empty(),
        "user1's stale ask should have been cancelled by the drift check"
    );

    // user2 filled 1 unit against user3 at $2,400. The stale $2,199 ask
    // was cancelled without filling, so user2's size-2 order is only
    // half-filled.
    let user2_state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user2.address(),
        })
        .should_succeed()
        .unwrap();
    let pos = user2_state
        .positions
        .get(&pair)
        .expect("user2 has a long position");
    assert_eq!(pos.size, Quantity::new_int(1));
    assert_eq!(pos.entry_price, UsdPrice::new_int(2_400));
}
