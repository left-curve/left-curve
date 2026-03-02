use {
    dango_genesis::Contracts,
    dango_testing::{TestOption, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        oracle::{self, PriceSource},
        perps::{self, PairParam, Param, QueryOrdersByUserResponse, UserState},
    },
    grug::{
        Addressable, Coins, Denom, Duration, NumberConst, QuerierExt, ResultExt, Timestamp,
        Udec128, Uint128, btree_map,
    },
    std::collections::BTreeSet,
};

fn pair_id() -> Denom {
    "perp/ethusd".parse().unwrap()
}

/// Return the genesis-default global params (mirrors `PerpsOption::preset_test()`).
fn default_param() -> Param {
    Param {
        taker_fee_rate: Dimensionless::new_permille(1), // 0.1%
        maker_fee_rate: Dimensionless::ZERO,
        liquidation_fee_rate: Dimensionless::new_permille(10), // 1%
        vault_cooldown_period: Duration::from_days(1),
        max_unlocks: 10,
        max_open_orders: 100,
        funding_period: Duration::from_hours(1),
        adl_operators: BTreeSet::new(),
        vault_total_weight: Dimensionless::ZERO,
    }
}

/// Return the genesis-default pair params (mirrors `PerpsOption::preset_test()`).
fn default_pair_param() -> PairParam {
    PairParam {
        initial_margin_ratio: Dimensionless::new_permille(100), // 10%
        maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
        tick_size: UsdPrice::new_int(1),
        max_abs_oi: Quantity::new_int(1_000_000),
        ..PairParam::new_mock()
    }
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

/// Covers: partial liquidation filled on order book, no bad debt.
///
/// Liquidation closes only enough of the position to cover the maintenance
/// margin deficit, not the entire position.
///
/// | Step | Action                                          | Key numbers                                                                                                        | Assert                                                                                 |
/// | ---- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
/// | 1    | Trader margin = $3,000; vault margin = $100,000 | —                                                                                                                  | —                                                                                      |
/// | 2    | Maker places ask: 5 ETH @ $2,000                | —                                                                                                                  | —                                                                                      |
/// | 3    | Trader market buys 5 ETH                        | fee = $10; margin = $2,990; position: 5 long @ $2,000                                                              | —                                                                                      |
/// | 4    | Oracle drops to $1,450                          | PnL = 5x($1,450-$2,000) = -$2,750; equity = $240; MM = 5x$1,450x5% = $362.50                                       | equity < MM -> liquidatable                                                            |
/// | 5    | Bidder places bid: 5 ETH @ $1,450               | —                                                                                                                  | —                                                                                      |
/// | 6    | Liquidate trader                                | deficit = $122.50; close ~1.689655 ETH via book; liq_fee ~$24.50; margin after ~$2,036.19; ~3.31 ETH position stays | trader position reduced; trader margin ~$2,036; vault margin += ~$24.50; bidder filled |
#[test]
fn liquidation_on_order_book() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    // Register oracle prices: ETH = $2,000, USDC = $1.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: Fund vault and trader.
    // LP (user4) deposits $100,000 USDC and adds $100,000 liquidity to the vault.
    // Trader (user1) deposits $3,000 USDC.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::AddLiquidity {
                amount: UsdValue::new_int(100_000),
                min_shares_to_mint: None,
            },
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(3_000_000_000)).unwrap(),
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
                size: Quantity::new_int(-5), // sell / ask
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
    // Step 3: Trader (user1) market buys 5 ETH.
    // Fee = 5 * $2,000 * 0.1% = $10.  Margin after = $3,000 - $10 = $2,990.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5), // buy
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // Verify position: 5 ETH long @ $2,000, margin = $2,990.
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
        UsdValue::new_int(2_990),
        "margin should be $2,990 after $10 fee"
    );

    // -------------------------------------------------------------------------
    // Step 4: Oracle drops to $1,450.
    // PnL = 5 * ($1,450 - $2,000) = -$2,750; equity = $2,990 - $2,750 = $240.
    // MM = 5 * $1,450 * 5% = $362.50; equity < MM -> liquidatable.
    // -------------------------------------------------------------------------
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_450);

    // -------------------------------------------------------------------------
    // Step 5: Bidder (user3) deposits $10,000 USDC and places bid: 5 ETH @ $1,450.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5), // buy / bid
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_450),
                    post_only: true,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 6: Liquidate trader (user1).
    //
    // Partial liquidation: deficit = MM - equity = $362.50 - $240 = $122.50
    // close_amount = $122.50 / ($1,450 * 5%) = 1.689655 ETH
    //
    // Matched against bidder's bid at $1,450 (zero taker/maker fee for liq fills).
    // Realized PnL = 1.689655 * ($1,450 - $2,000) = -$929.310250
    // Closed notional = 1.689655 * $1,450 = $2,449.999750
    // Liq fee = $2,449.999750 * 1% = $24.499997
    // Trader margin after = $2,990 - $929.310250 - $24.499997 = $2,036.189753
    // -------------------------------------------------------------------------

    // Capture vault margin before liquidation.
    let vault_state_before: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_margin_before = vault_state_before.unwrap().margin;

    // Anyone can call Liquidate.
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Liquidate {
                user: accounts.user1.address(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Trader position should be reduced from 5 to ~3.310345 ETH (partial close).
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let state = state.unwrap();
    let pos = state
        .positions
        .get(&pair)
        .expect("trader should still have a reduced ETH position");
    // 5.000000 - 1.689655 = 3.310345 ETH remaining.
    assert_eq!(
        pos.size,
        Quantity::new_raw(3_310_345),
        "trader should have ~3.31 ETH after partial liquidation"
    );

    // Trader margin = $2,036.189753 (raw 2_036_189_753).
    assert_eq!(
        state.margin,
        UsdValue::new_raw(2_036_189_753),
        "trader margin should be ~$2,036.19 after partial liquidation"
    );

    // Vault margin should have increased by the liquidation fee (~$24.50).
    let vault_state_after: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_margin_after = vault_state_after.unwrap().margin;
    assert_eq!(
        vault_margin_after - vault_margin_before,
        UsdValue::new_raw(24_499_997),
        "vault margin should increase by ~$24.50 liquidation fee"
    );

    // Bidder (user3) should have ~1.689655 ETH long @ $1,450.
    let bidder_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed();
    let bidder_state = bidder_state.unwrap();
    let bidder_pos = bidder_state
        .positions
        .get(&pair)
        .expect("bidder should have ETH position");
    assert_eq!(
        bidder_pos.size,
        Quantity::new_raw(1_689_655),
        "bidder should have ~1.69 ETH long from partial fill"
    );
    assert_eq!(
        bidder_pos.entry_price,
        UsdPrice::new_int(1_450),
        "bidder entry price should be $1,450"
    );
}

/// Covers: vault backstop liquidation → bad debt → ADL recovery.
///
/// | Step | Action                                    | Assert                                              |
/// | ---- | ----------------------------------------- | --------------------------------------------------- |
/// | 1    | LP deposits $1,000, adds $1,000 liquidity | vault margin = $1,000                               |
/// | 2    | Trader A deposits $1,100                  | margin = $1,100                                     |
/// | 3    | Maker deposits $10k, places ask 5 ETH@$2k | —                                                   |
/// | 4    | Trader A market buys 5 ETH                | fee=$10; margin=$1,090; 5 long @$2,000              |
/// | 5    | Trader B deposits $10k                    | —                                                   |
/// | 6    | Maker places bid: 5 ETH @ $2,000          | —                                                   |
/// | 7    | Trader B market sells 5 ETH               | fee=$10; margin=$9,990; 5 short @$2,000             |
/// | 8    | Oracle → $1,450                           | Trader A: PnL=-$2,750, equity=-$1,660               |
/// | 9    | Liquidate Trader A                        | No bids → vault backstops; bad debt=$1,660          |
/// | 10   | Verify vault deficit                      | vault margin = -$660                                |
/// | 11   | Configure: set adl_operators              | —                                                   |
/// | 12   | ADL Trader B                              | Trader B margin=$12,080; vault margin=$0            |
#[test]
fn liquidation_bad_debt_then_adl() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    // Register oracle prices: ETH = $2,000, USDC = $1.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: LP (user4) deposits $1,000 and adds $1,000 liquidity.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(1_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::AddLiquidity {
                amount: UsdValue::new_int(1_000),
                min_shares_to_mint: None,
            },
            Coins::new(),
        )
        .should_succeed();

    // Verify vault margin = $1,000.
    let vault_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    assert_eq!(vault_state.unwrap().margin, UsdValue::new_int(1_000));

    // -------------------------------------------------------------------------
    // Step 2: Trader A (user1) deposits $1,100 USDC.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(1_100_000_000)).unwrap(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 3: Maker (user2) deposits $10,000, places ask: 5 ETH @ $2,000.
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
                size: Quantity::new_int(-5),
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
    // Step 4: Trader A market buys 5 ETH.
    // Fee = 5 * $2,000 * 0.1% = $10. Margin = $1,100 - $10 = $1,090.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    assert_eq!(state.unwrap().margin, UsdValue::new_int(1_090));

    // -------------------------------------------------------------------------
    // Step 5: Trader B (user3) deposits $10,000.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 6: Maker places bid: 5 ETH @ $2,000.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
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
    // Step 7: Trader B market sells 5 ETH.
    // Fee = 5 * $2,000 * 0.1% = $10. Margin = $10,000 - $10 = $9,990.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed();
    assert_eq!(state.unwrap().margin, UsdValue::new_int(9_990));

    // -------------------------------------------------------------------------
    // Step 8: Oracle → $1,450.
    // Trader A PnL = 5 * ($1,450 - $2,000) = -$2,750; equity = $1,090 - $2,750 = -$1,660.
    // -------------------------------------------------------------------------
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_450);

    // -------------------------------------------------------------------------
    // Step 9: Liquidate Trader A.
    // No bids on book → vault backstops at oracle price.
    // PnL = -$2,750; margin after PnL = $1,090 - $2,750 = -$1,660.
    // Liq fee capped at remaining = $0 (margin already negative).
    // Bad debt = $1,660 → user margin floors to $0, vault absorbs.
    // Vault margin = $1,000 - $1,660 = -$660.
    // Vault inherits 5 long @ $1,450.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Liquidate {
                user: accounts.user1.address(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Trader A should have no positions and $0 margin.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    // User state is empty (margin=0, no positions) — may be pruned.
    assert!(
        state.is_none() || state.as_ref().unwrap().positions.is_empty(),
        "Trader A should have no positions"
    );

    // -------------------------------------------------------------------------
    // Step 10: Verify vault deficit.
    // Vault received $10 taker fee from each of steps 4 and 7, so
    // vault margin before liq = $1,000 + $10 + $10 = $1,020.
    // Bad debt = $1,660 → vault margin = $1,020 - $1,660 = -$640.
    // -------------------------------------------------------------------------
    let vault_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_state = vault_state.unwrap();
    assert_eq!(
        vault_state.margin,
        UsdValue::new_int(-640),
        "vault margin should be -$640 (bad debt, after $20 taker fees)"
    );
    // Vault should have 5 long @ $1,450.
    let vault_pos = vault_state
        .positions
        .get(&pair)
        .expect("vault should have backstop position");
    assert_eq!(vault_pos.size, Quantity::new_int(5));
    assert_eq!(vault_pos.entry_price, UsdPrice::new_int(1_450));

    // -------------------------------------------------------------------------
    // Step 11: Configure: add owner as ADL operator.
    // -------------------------------------------------------------------------
    let owner_addr = accounts.owner.address();
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Configure {
                param: Param {
                    adl_operators: BTreeSet::from([owner_addr]),
                    ..default_param()
                },
                pair_params: btree_map! { pair.clone() => default_pair_param() },
            },
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 12: ADL Trader B.
    // Trader B: 5 short @ $2,000, margin = $9,990.
    // PnL = -5 * ($1,450 - $2,000) = +$2,750 (profit).
    // Deficit = $640; forfeited = min($2,750, $640) = $640.
    // Credit = $2,750 - $640 = $2,110.
    // Trader B margin = $9,990 + $2,110 = $12,100.
    // Vault margin = -$640 + $640 = $0.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Deleverage {
                user: accounts.user3.address(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Trader B: no positions, margin = $12,080.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed();
    let state = state.unwrap();
    assert!(
        state.positions.is_empty(),
        "Trader B should have no positions after ADL"
    );
    assert_eq!(
        state.margin,
        UsdValue::new_int(12_100),
        "Trader B margin should be $12,100 after ADL"
    );

    // Vault margin = $0.
    let vault_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_state = vault_state.unwrap();
    assert_eq!(
        vault_state.margin,
        UsdValue::ZERO,
        "vault margin should be $0 after ADL"
    );
}

/// Covers: add liquidity → vault trades → realized PnL reflected in share
/// price → correct withdrawal amounts.
///
/// | Step | Action                                           | Assert                                          |
/// | ---- | ------------------------------------------------ | ----------------------------------------------- |
/// | 1    | LP deposits $10k, adds $5k liquidity             | vault margin=$5,000; shares minted              |
/// | 2    | Configure: enable vault MM (weight=1, spread=5%) | —                                               |
/// | 3    | OnOracleUpdate → vault places bid+ask             | vault has orders on book                        |
/// | 4    | Taker deposits $10k, sells into vault's bid       | vault gets long position                        |
/// | 5    | Oracle → $2,200                                  | vault's long has unrealized profit              |
/// | 6    | OnOracleUpdate → vault refreshes orders           | —                                               |
/// | 7    | Taker closes (buys back via vault ask)            | vault realizes PnL; vault margin increases      |
/// | 8    | LP removes half shares                           | unlock reflects realized PnL in share price     |
/// | 9    | LP removes remaining shares                      | unlock reflects realized PnL                    |
/// | 10   | Advance time past cooldown                       | unlocks credited to LP margin                   |
/// | 11   | Verify total withdrawn ≈ $5k + vault profit      | —                                               |
#[test]
fn vault_lp_lifecycle() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    // Register oracle prices: ETH = $2,000, USDC = $1.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Step 1: LP (user1) deposits $10,000 and adds $5,000 liquidity.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::AddLiquidity {
                amount: UsdValue::new_int(5_000),
                min_shares_to_mint: None,
            },
            Coins::new(),
        )
        .should_succeed();

    let lp_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let lp_state = lp_state.unwrap();
    let total_shares = lp_state.vault_shares;
    assert!(total_shares > Uint128::ZERO, "LP should have shares");
    assert_eq!(lp_state.margin, UsdValue::new_int(5_000), "LP margin = $5k");

    let vault_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    assert_eq!(
        vault_state.unwrap().margin,
        UsdValue::new_int(5_000),
        "vault margin = $5,000"
    );

    // -------------------------------------------------------------------------
    // Step 2: Configure: enable vault market-making.
    // vault_total_weight = 1, vault_liquidity_weight = 1, vault_half_spread = 5%,
    // vault_max_quote_size = 2 ETH (small to keep math tractable).
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Configure {
                param: Param {
                    vault_total_weight: Dimensionless::new_int(1),
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair.clone() => PairParam {
                        vault_liquidity_weight: Dimensionless::new_int(1),
                        vault_half_spread: Dimensionless::new_permille(50), // 5%
                        vault_max_quote_size: Quantity::new_int(2),
                        ..default_pair_param()
                    },
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 3: Call OnOracleUpdate so the vault places bid+ask.
    // -------------------------------------------------------------------------
    suite.make_empty_block();

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::OnOracleUpdate {},
            Coins::new(),
        )
        .should_succeed();

    // Vault should have orders on the book.
    let vault_orders: QueryOrdersByUserResponse = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();
    assert!(
        !vault_orders.bids.is_empty(),
        "vault should have a bid on the book"
    );
    assert!(
        !vault_orders.asks.is_empty(),
        "vault should have an ask on the book"
    );

    // Vault bid = $2,000 * (1 - 5%) = $1,900, ask = $2,000 * (1 + 5%) = $2,100.
    let vault_bid_price = vault_orders.bids[0].limit_price;
    assert_eq!(vault_bid_price, UsdPrice::new_int(1_900));
    let vault_bid_size = vault_orders.bids[0].size;

    // -------------------------------------------------------------------------
    // Step 4: Taker (user2) deposits $10k, market sells into vault's bid.
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
                size: vault_bid_size.checked_neg().unwrap(), // sell
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // Vault should now have a long position.
    let vault_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_state = vault_state.unwrap();
    let vault_pos = vault_state
        .positions
        .get(&pair)
        .expect("vault should have a position");
    assert!(vault_pos.size.is_positive(), "vault should be long");
    let vault_long_size = vault_pos.size;
    let vault_margin_after_buy = vault_state.margin;

    // -------------------------------------------------------------------------
    // Step 5: Oracle → $2,200. Vault's long has unrealized profit.
    // -------------------------------------------------------------------------
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_200);

    // -------------------------------------------------------------------------
    // Step 6: OnOracleUpdate at $2,200 → vault refreshes orders.
    // -------------------------------------------------------------------------
    suite.make_empty_block();

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::OnOracleUpdate {},
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 7: Taker buys back from vault's ask → vault realizes PnL.
    // Vault ask ≈ $2,200 * 1.05 = $2,310.
    // -------------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: vault_long_size, // buy same amount (closes taker's short)
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    // Vault should have realized PnL → margin increased above pre-trade level.
    let vault_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_state = vault_state.unwrap();
    assert!(
        vault_state.margin > vault_margin_after_buy,
        "vault margin should increase after realized PnL"
    );

    // -------------------------------------------------------------------------
    // Step 8: LP removes half shares. Share price includes realized profit.
    // -------------------------------------------------------------------------
    let half_shares = total_shares / Uint128::new(2);

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::RemoveLiquidity {
                shares_to_burn: half_shares,
            },
            Coins::new(),
        )
        .should_succeed();

    let lp_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let lp_state = lp_state.unwrap();
    let unlock1_amount = lp_state.unlocks.back().unwrap().amount_to_release;

    // The unlock should be > $2,500 (half of original $5k) because the vault
    // realized profits from the long position.
    assert!(
        unlock1_amount > UsdValue::new_int(2_500),
        "first unlock ({unlock1_amount}) should exceed $2,500 due to realized PnL"
    );

    // -------------------------------------------------------------------------
    // Step 9: LP removes remaining shares.
    // -------------------------------------------------------------------------
    let remaining_shares = lp_state.vault_shares;

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::RemoveLiquidity {
                shares_to_burn: remaining_shares,
            },
            Coins::new(),
        )
        .should_succeed();

    let lp_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let lp_state = lp_state.unwrap();
    assert_eq!(
        lp_state.unlocks.len(),
        2,
        "LP should have 2 pending unlocks"
    );
    let unlock2_amount = lp_state.unlocks.back().unwrap().amount_to_release;

    // -------------------------------------------------------------------------
    // Step 10: Advance time past cooldown (1 day) so cron processes unlocks.
    // -------------------------------------------------------------------------
    suite.increase_time(Duration::from_days(2));

    // -------------------------------------------------------------------------
    // Step 11: Verify total withdrawn reflects original $5k + vault profits.
    // -------------------------------------------------------------------------
    let lp_state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();
    let lp_state = lp_state.unwrap();

    // Unlocks should have been processed (queue empty).
    assert!(
        lp_state.unlocks.is_empty(),
        "all unlocks should be processed"
    );

    // Total credited = unlock1 + unlock2.
    let total_unlocked = unlock1_amount.checked_add(unlock2_amount).unwrap();
    assert!(
        total_unlocked > UsdValue::new_int(5_000),
        "total withdrawn ({total_unlocked}) should exceed original $5,000 deposit"
    );

    // The LP's margin should now include the original $5k trading margin
    // plus the unlocked amounts.
    assert!(
        lp_state.margin > UsdValue::new_int(10_000),
        "LP margin ({}) should exceed original $10k (unlocks credited)",
        lp_state.margin
    );
}
