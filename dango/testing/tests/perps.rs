use {
    dango_genesis::Contracts,
    dango_testing::{TestOption, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        oracle::{self, PriceSource},
        perps::{self, QueryOrdersByUserResponse, UserState},
    },
    grug::{
        Addressable, Coins, Denom, NumberConst, QuerierExt, ResultExt, Timestamp, Udec128, Uint128,
        btree_map,
    },
};

fn pair_id() -> Denom {
    "perp/ethusd".parse().unwrap()
}

/// Register fixed oracle prices for the perps pair and settlement currency.
fn register_oracle_prices(
    suite: &mut dango_testing::TestSuite<grug_app::NaiveProposalPreparer>,
    accounts: &mut dango_testing::TestAccounts,
    contracts: &Contracts,
    eth_price: u128,
) {
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: usdc::DECIMAL as u8,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
                pair_id() => PriceSource::Fixed {
                    humanized_price: Udec128::new(eth_price),
                    precision: 0,
                    timestamp: Timestamp::from_nanos(u128::MAX),
                },
            }),
            Coins::new(),
        )
        .should_succeed();
}

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
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Verify trader's margin = $10,000.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let state = state.unwrap();
    assert_eq!(state.margin, UsdValue::new_int(10_000));

    // -------------------------------------------------------------------------
    // Step 2: Maker (user2) deposits $10,000 USDC and places ask: 10 ETH @ $2,000.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10), // sell / ask
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: true,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // Verify ask exists on the book.
    let orders: QueryOrdersByUserResponse = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();
    assert_eq!(orders.asks.len(), 1, "maker should have 1 ask");

    // -------------------------------------------------------------------------
    // Step 3: Trader market buys 10 ETH.
    // Fee = 10 * $2,000 * 0.1% = $20.  Margin after = $10,000 - $20 = $9,980.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10), // buy
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // Verify position and margin.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let state = state.unwrap();
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
    let orders: QueryOrdersByUserResponse = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();
    assert!(
        orders.asks.is_empty(),
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
            &perps::ExecuteMsg::Withdraw {
                amount: UsdValue::new_int(7_000),
            },
            Coins::new(),
        )
        .should_succeed();

    // Verify margin = $9,980 - $7,000 = $2,980.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let state = state.unwrap();
    assert_eq!(state.margin, UsdValue::new_int(2_980));

    // -------------------------------------------------------------------------
    // Step 5: Trader withdraws $2,000 (should fail).
    // available = $2,980 - IM($2,000) = $980 < $2,000.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Withdraw {
                amount: UsdValue::new_int(2_000),
            },
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
            &perps::ExecuteMsg::Deposit {},
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
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5), // sell / ask 5 ETH
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: true,
                },
                reduce_only: false,
            },
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
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10), // buy 10 ETH
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: false,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // Verify position: 5 ETH long @ $2,000, margin = $9,990.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let state = state.unwrap();
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
    let orders: QueryOrdersByUserResponse = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(orders.bids.len(), 1, "trader should have 1 resting bid");

    // -------------------------------------------------------------------------
    // Step 4: Trader cancels the resting order.
    // -------------------------------------------------------------------------
    let order_id = orders.bids[0].order_id;

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::CancelOrder(perps::CancelOrderRequest::One(order_id)),
            Coins::new(),
        )
        .should_succeed();

    // Verify reserved_margin = $0 and open_order_count = 0.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let state = state.unwrap();
    assert_eq!(
        state.reserved_margin,
        UsdValue::ZERO,
        "reserved_margin should be $0 after cancel"
    );
    assert_eq!(state.open_order_count, 0, "should have 0 open orders");

    // Verify bid removed from book.
    let orders: QueryOrdersByUserResponse = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert!(orders.bids.is_empty(), "bids should be empty after cancel");

    // Verify position unchanged: still 5 ETH long @ $2,000.
    let pos = state
        .positions
        .get(&pair)
        .expect("should still have ETH position");
    assert_eq!(pos.size, Quantity::new_int(5), "should still be 5 ETH long");
    assert_eq!(pos.entry_price, UsdPrice::new_int(2_000));
}
