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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
                    post_only: true,
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
                    max_slippage: Dimensionless::ONE,
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
                    post_only: true,
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
                    post_only: false,
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
                    post_only: true,
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
                    post_only: true,
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
                    post_only: false,
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
                    post_only: true,
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
                    max_slippage: Dimensionless::ONE,
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
                    post_only: true,
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
                    max_slippage: Dimensionless::ONE,
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit {}),
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
                    post_only: true,
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
                    max_slippage: Dimensionless::ONE,
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
