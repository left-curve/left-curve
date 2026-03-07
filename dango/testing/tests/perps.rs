use {
    dango_genesis::Contracts,
    dango_testing::{TestOption, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        oracle::{self, PriceSource, QueryPriceRequest},
        perps::{self, LiquidityDepthResponse, PairParam, Param, UserState},
    },
    grug::{
        Addressable, Binary, ByteArray, Coins, Denom, Duration, NonEmpty, NumberConst, QuerierExt,
        ResultExt, Timestamp, Udec128, Uint128, btree_map, btree_set, concat,
    },
    grug_app::CONTRACT_NAMESPACE,
    pyth_types::{Channel, LeEcdsaMessage},
    std::{collections::BTreeMap, str::FromStr},
};

fn pair_id() -> Denom {
    "perp/ethusd".parse().unwrap()
}

/// Return the genesis-default global params (mirrors `PerpsOption::preset_test()`).
fn default_param() -> Param {
    Param {
        base_taker_fee_rate: Dimensionless::new_permille(1), // 0.1%
        base_maker_fee_rate: Dimensionless::ZERO,
        tiered_taker_fee_rate: BTreeMap::new(),
        tiered_maker_fee_rate: BTreeMap::new(),
        protocol_fee_rate: Dimensionless::ZERO,
        liquidation_fee_rate: Dimensionless::new_permille(10), // 1%
        vault_cooldown_period: Duration::from_days(1),
        max_unlocks: 10,
        max_open_orders: 100,
        funding_period: Duration::from_hours(1),
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
    let orders = suite
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
    let orders = suite
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
    let orders = suite
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
    let orders = suite
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
    let vault_state_before = suite
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
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
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

    // Insurance fund should have received the liquidation fee (~$24.50).
    let global_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_state.insurance_fund,
        UsdValue::new_raw(24_499_997),
        "insurance fund should receive ~$24.50 liquidation fee"
    );

    // Vault margin should be unchanged (fee goes to insurance fund, not vault).
    let vault_state_after: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed();
    let vault_margin_after = vault_state_after.unwrap().margin;

    assert_eq!(
        vault_margin_after, vault_margin_before,
        "vault margin should be unchanged (fee goes to insurance fund)"
    );

    // Bidder (user3) should have ~1.689655 ETH long @ $1,450.
    let bidder_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();
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

/// Covers: liquidation with ADL and insurance fund.
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
/// | 9    | Liquidate Trader A                        | No bids → ADL against Trader B at bankruptcy price  |
/// | 10   | Verify results                            | Trader B position reduced; insurance fund updated   |
#[test]
fn liquidation_with_adl() {
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

    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();

    assert_eq!(state.margin, UsdValue::new_int(9_990));

    // -------------------------------------------------------------------------
    // Step 8: Oracle → $1,450.
    // Trader A: 5 long @$2,000.
    // PnL = 5 * ($1,450 - $2,000) = -$2,750; equity = $1,090 - $2,750 = -$1,660.
    // -------------------------------------------------------------------------

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_450);

    // -------------------------------------------------------------------------
    // Step 9: Liquidate Trader A.
    //
    // No bids on book → ADL against Trader B (most profitable short).
    //
    // Bankruptcy price for Trader A's long:
    //   bp = oracle_price - equity / |size|
    //      = $1,450 - (-$1,660) / 5  (equity is negative)
    //      = $1,450 + $332 = $1,782
    //
    // ADL: close Trader A's 5 long and Trader B's 5 short at $1,782.
    //
    // Trader A PnL at bp = 5 * ($1,782 - $2,000) = -$1,090.
    //   margin after = $1,090 - $1,090 = $0. No bad debt.
    //   Liq fee = min(0, ...) = $0 (no remaining margin).
    //
    // Trader B PnL at bp = -5 * ($1,782 - $2,000) = +$1,090.
    //   margin after = $9,990 + $1,090 = $11,080.
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
    let state = suite
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
    // Step 10: Verify Trader B's position was ADL'd.
    // -------------------------------------------------------------------------
    let state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();

    assert!(
        state.positions.is_empty(),
        "Trader B should have no positions after ADL"
    );

    // Trader B PnL at bankruptcy price: -5 * ($1,782 - $2,000) = +$1,090.
    // Margin = $9,990 + $1,090 = $11,080.
    assert_eq!(
        state.margin,
        UsdValue::new_int(11_080),
        "Trader B margin should be $11,080 after ADL at bankruptcy price"
    );

    // Vault should be unaffected — no backstop, no bad debt.
    let vault_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

    // Vault received $10 taker fee from each of steps 4 and 7 = $20.
    // Vault margin = $1,000 + $20 = $1,020. No bad debt absorbed.
    assert_eq!(
        vault_state.margin,
        UsdValue::new_int(1_020),
        "vault margin should be $1,020 (initial + taker fees, no bad debt)"
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

    let lp_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

    assert!(
        lp_state.vault_shares > Uint128::ZERO,
        "LP should have shares"
    );
    assert_eq!(lp_state.margin, UsdValue::new_int(5_000), "LP margin = $5k");

    let total_shares = lp_state.vault_shares;

    let vault_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

    assert_eq!(
        vault_state.margin,
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
    let vault_orders = suite
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
    assert_eq!(vault_orders.bids[0].limit_price, UsdPrice::new_int(1_900));

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
    let vault_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();
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
    let vault_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

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

    let lp_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
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

    let lp_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

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

    let lp_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();

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

/// Verify that feeding Pyth prices triggers `OnOracleUpdate` (placing vault
/// orders), and that when `OnOracleUpdate` fails the oracle price update is
/// **not** reverted while the perps state changes from the failed call are
/// rolled back.
#[test]
fn oracle_triggers_on_oracle_update() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    let pair = pair_id();

    // -------------------------------------------------------------------------
    // Setup: Register Pyth price source for the perps pair + Fixed USDC source.
    // Genesis already registers the LAZER trusted signer with Timestamp::MAX.
    // We override USDC to Fixed so we don't need a USDC Pyth feed in this test.
    // -------------------------------------------------------------------------

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
                pair.clone() => PriceSource::Pyth {
                    id: 2,
                    precision: 18,
                    channel: Channel::RealTime,
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Setup: Deposit USDC and add vault liquidity (follows vault_lp_lifecycle).
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

    // -------------------------------------------------------------------------
    // Setup: Configure vault market-making weights.
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

    // Vault should have no orders before any price is fed.
    let vault_orders_0 = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();

    assert!(
        vault_orders_0.bids.is_empty(),
        "vault should have no bids before feeding prices"
    );
    assert!(
        vault_orders_0.asks.is_empty(),
        "vault should have no asks before feeding prices"
    );

    // -------------------------------------------------------------------------
    // Step 1: Feed price #1. This triggers OnOracleUpdate via the submessage
    // and the vault should place bid+ask orders.
    // -------------------------------------------------------------------------

    let message1 = LeEcdsaMessage {
        payload: Binary::from_str(
            "ddPHkyAnhCsRTAYAAQICAAAAAgDLzMJzLwAAAAT4/wcAAAACAPnb9QUAAAAABPj/",
        )
        .unwrap(),
        signature: ByteArray::from_str(
            "HJt9BJHEBuX0VhWDIjldnfwIYO9ufenGCVTMhQUwxhoYiX+TVDSqbNdQpXsRilNrS9Z7q/ET8obCBM9c97DmcQ==",
        )
        .unwrap(),
        recovery_id: 1,
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message1])),
            Coins::new(),
        )
        .should_succeed();

    // Oracle price should be set for the perps pair.
    let price1 = suite
        .query_wasm_smart(contracts.oracle, QueryPriceRequest {
            denom: pair.clone(),
        })
        .unwrap();

    assert!(
        price1.humanized_price > Udec128::ZERO,
        "oracle price should be set after feeding"
    );

    // Vault should have orders on the book (placed by OnOracleUpdate).
    let vault_orders_1 = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();

    assert_eq!(
        vault_orders_1.bids.len(),
        1,
        "vault should have exactly 1 bid after OnOracleUpdate"
    );
    assert_eq!(
        vault_orders_1.asks.len(),
        1,
        "vault should have exactly 1 ask after OnOracleUpdate"
    );
    assert_eq!(
        vault_orders_1.bids[0].pair_id, pair,
        "bid should be for the perps pair"
    );
    assert_eq!(
        vault_orders_1.asks[0].pair_id, pair,
        "ask should be for the perps pair"
    );

    // bid = floor(oracle * 0.95) = floor(2038.056 * 0.95) = $1,936
    // ask = ceil(oracle * 1.05) = ceil(2038.056 * 1.05) = $2,140
    //
    // Note that we use $1 tick size in the testing setup. It's not a sensible
    // tick size for production, but it simplifies assertions like this.
    assert_eq!(vault_orders_1.bids[0].limit_price, UsdPrice::new_int(1_936));
    assert_eq!(vault_orders_1.asks[0].limit_price, UsdPrice::new_int(2_140));

    // |size| = min(half_margin / (oracle * IMR), vault_max_quote_size)
    //        = min(2500 / (2038.056 * 0.1), 2) = min(12.27, 2) = 2
    assert_eq!(vault_orders_1.bids[0].size, Quantity::new_int(2));
    assert_eq!(vault_orders_1.asks[0].size, Quantity::new_int(-2));

    // -------------------------------------------------------------------------
    // Step 2: Corrupt perps PARAM storage so OnOracleUpdate will fail on the
    // next invocation (deserialization error).
    // -------------------------------------------------------------------------

    suite.app.db.with_state_storage_mut(|storage| {
        let ns = concat(CONTRACT_NAMESPACE, contracts.perps.as_ref());
        let full_key = concat(&ns, b"param");
        storage.write(&full_key, b"garbled");
    });

    // -------------------------------------------------------------------------
    // Step 3: Feed price #2. The oracle update itself succeeds (reply_on_error
    // catches the perps failure), but OnOracleUpdate rolls back its state
    // changes, so vault orders remain unchanged.
    // -------------------------------------------------------------------------

    let message2 = LeEcdsaMessage {
        payload: Binary::from_str(
            "ddPHk0DIiysRTAYAAQICAAAAAgD3e8JzLwAAAAT4/wcAAAACADDZ9QUAAAAABPj/",
        )
        .unwrap(),
        signature: ByteArray::from_str(
            "kToxd5mWk50/kezThZVzUf7cFIJ7t/fpDs5TboBop5Av9MgXhfcwsFPxtPwXkN7zwxul1U+Z/EOVje4HW53BBg==",
        )
        .unwrap(),
        recovery_id: 0,
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::FeedPrices(NonEmpty::new_unchecked(vec![message2])),
            Coins::new(),
        )
        .should_succeed();

    // Oracle price should have been updated (new timestamp or value).
    let price2 = suite
        .query_wasm_smart(contracts.oracle, QueryPriceRequest {
            denom: pair.clone(),
        })
        .unwrap();

    assert!(
        price2.timestamp >= price1.timestamp,
        "oracle price should be updated despite OnOracleUpdate failure"
    );

    // Vault orders should be unchanged — the failed OnOracleUpdate rolled back
    // any state changes it attempted (cancel + re-place). Compare order IDs to
    // prove these are the exact same orders, not new ones at the same price.
    let vault_orders_2 = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();

    assert!(
        vault_orders_1
            .bids
            .iter()
            .zip(vault_orders_2.bids.iter())
            .all(|(a, b)| a.order_id == b.order_id),
        "bid order IDs should be unchanged after failed OnOracleUpdate"
    );
    assert!(
        vault_orders_1
            .asks
            .iter()
            .zip(vault_orders_2.asks.iter())
            .all(|(a, b)| a.order_id == b.order_id),
        "ask order IDs should be unchanged after failed OnOracleUpdate"
    );
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
            &perps::ExecuteMsg::Configure {
                param: default_param(),
                pair_params: btree_map! {
                    pair.clone() => PairParam {
                        bucket_sizes: btree_set! { UsdPrice::new_int(100) },
                        ..default_pair_param()
                    },
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // Deposit margin for both users.
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
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
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
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-3),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: true,
                },
                reduce_only: false,
            },
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
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: false,
                },
                reduce_only: false,
            },
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
            &perps::ExecuteMsg::CancelOrder(perps::CancelOrderRequest::All),
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
            &perps::ExecuteMsg::Configure {
                param: Param {
                    protocol_fee_rate: Dimensionless::new_percent(20),
                    ..default_param()
                },
                pair_params: btree_map! {
                    pair.clone() => default_pair_param(),
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // Deposit for maker (user2) and taker (user1).
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Deposit {},
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    // --- Fill 1: Maker places ask 10 ETH @ $2,000, taker market buys ---

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: true,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
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
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-10),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    post_only: true,
                },
                reduce_only: false,
            },
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(10),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            },
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
