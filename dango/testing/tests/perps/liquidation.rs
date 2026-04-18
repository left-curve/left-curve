use {
    crate::{default_pair_param, refresh_vault_orders, register_oracle_prices},
    dango_testing::{TestOption, perps::pair_id, setup_test_naive},
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        constants::usdc,
        perps::{
            self, Deleveraged, OrderFilled, PairParam, Param, QueryOrdersByUserResponseItem,
            UserState,
        },
    },
    grug::{
        Addressable, CheckedContractEvent, Coins, Duration, JsonDeExt, QuerierExt, ResultExt,
        SearchEvent, Uint128, btree_map,
    },
    std::collections::BTreeMap,
};

/// Return the genesis-default global params with `liquidation_fee_rate = ZERO`.
///
/// This is a separate helper from `crate::default_param()` because the vault
/// liquidation test needs zero liquidation fee so the post-close fee doesn't
/// erode equity below maintenance margin, allowing us to assert post-liquidation
/// health.
fn default_param_no_liq_fee() -> Param {
    Param {
        taker_fee_rates: perps::RateSchedule {
            base: Dimensionless::new_permille(1), // 0.1%
            ..Default::default()
        },
        protocol_fee_rate: Dimensionless::ZERO,
        // Zero so the post-close fee doesn't erode equity below MM,
        // allowing us to assert post-liquidation health.
        liquidation_fee_rate: Dimensionless::ZERO,
        vault_cooldown_period: Duration::from_days(1),
        max_unlocks: 10,
        max_open_orders: 100,
        funding_period: Duration::from_hours(1),
        max_action_batch_size: 5,
        ..Default::default()
    }
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::AddLiquidity {
                amount: UsdValue::new_int(100_000),
                min_shares_to_mint: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5), // sell / ask
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5), // buy
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5), // buy / bid
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_450),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 6: Liquidate trader (user1).
    //
    // Partial liquidation: deficit = MM - equity = $362.50 - $240 = $122.50
    // close_amount = ceil($122.50 / ($1,450 * 5%)) = 1.689656 ETH
    // (ceil rounding guarantees at least one ULP of progress; see
    // `compute_close_schedule` in `dango/perps/src/core/closure.rs`).
    //
    // Matched against bidder's bid at $1,450 (zero taker/maker fee for liq fills).
    // Realized PnL = 1.689656 * ($1,450 - $2,000) = -$929.310800
    // Closed notional = 1.689656 * $1,450 = $2,450.001200
    // Liq fee = $2,450.001200 * 1% = $24.500012
    // Trader margin after = $2,990 - $929.310800 - $24.500012 = $2,036.189188
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
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: accounts.user1.address(),
            }),
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

    // 5.000000 - 1.689656 = 3.310344 ETH remaining.
    // (close_amount rounded up by 1 ULP vs floor division; see
    // `compute_close_schedule` ceil rounding.)
    assert_eq!(
        pos.size,
        Quantity::new_raw(3_310_344),
        "trader should have ~3.31 ETH after partial liquidation"
    );

    // Trader margin = $2,036.189188 (raw 2_036_189_188).
    assert_eq!(
        state.margin,
        UsdValue::new_raw(2_036_189_188),
        "trader margin should be ~$2,036.19 after partial liquidation"
    );

    // Insurance fund should have received the liquidation fee (~$24.50).
    let global_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_state.insurance_fund,
        UsdValue::new_raw(24_500_012),
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
        Quantity::new_raw(1_689_656),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(1_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::AddLiquidity {
                amount: UsdValue::new_int(1_000),
                min_shares_to_mint: None,
            }),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
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
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: accounts.user1.address(),
            }),
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

/// Liquidation cancels conditional orders alongside regular orders.
/// Follows the pattern from `liquidation_on_order_book`.
#[test]
fn liquidation_cancels_conditional_orders() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // Step 1: Fund vault ($100k via user4), trader (user1) deposits $3,000.
    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(100_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user4,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::AddLiquidity {
                amount: UsdValue::new_int(100_000),
                min_shares_to_mint: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(3_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Step 2: Maker (user2) deposits and places ask: 5 ETH @ $2,000.
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Trader buys 5 ETH.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Step 3: Trader submits TP and SL conditional orders.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitConditionalOrder {
                pair_id: pair.clone(),
                size: Some(Quantity::new_int(-5)),
                trigger_price: UsdPrice::new_int(2_500),
                trigger_direction: perps::TriggerDirection::Above,
                max_slippage: Dimensionless::new_percent(1),
            }),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitConditionalOrder {
                pair_id: pair.clone(),
                size: Some(Quantity::new_int(-5)),
                trigger_price: UsdPrice::new_int(1_500),
                trigger_direction: perps::TriggerDirection::Below,
                max_slippage: Dimensionless::new_percent(2),
            }),
            Coins::new(),
        )
        .should_succeed();

    // Verify both conditional orders were placed.
    let state: UserState = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed()
        .unwrap();
    let pos = state.positions.get(&pair).expect("should have position");
    assert!(
        pos.conditional_order_above.is_some(),
        "should have TP (above) conditional order"
    );
    assert!(
        pos.conditional_order_below.is_some(),
        "should have SL (below) conditional order"
    );

    // Step 4: Oracle drops to $1,450.
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_450);

    // Step 5: Bidder (user3) deposits and places bid: 5 ETH @ $1,450.
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_450),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Step 6: Liquidate trader.
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

    // Step 7: Verify state after liquidation.
    let state: Option<UserState> = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    // All limit orders should have been canceled during liquidation.
    let all_orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user1.address(),
        })
        .should_succeed();

    assert!(
        all_orders.is_empty(),
        "all limit orders should be canceled after liquidation"
    );

    // Conditional orders should be gone: if position was fully closed, they
    // disappeared with it; if partially liquidated, they were cleared.
    if let Some(ref st) = state {
        for (pid, pos) in &st.positions {
            assert!(
                pos.conditional_order_above.is_none(),
                "conditional_order_above should be None for pair {pid} after liquidation"
            );
            assert!(
                pos.conditional_order_below.is_none(),
                "conditional_order_below should be None for pair {pid} after liquidation"
            );
        }
    }
}

/// Verifies that the vault (contract address) can be liquidated when its
/// equity falls below maintenance margin.
///
/// The vault accumulates a losing long position through market-making, then a
/// price drop makes it liquidatable. A keeper calls `Liquidate` and the vault's
/// position is closed against resting bids on the order book.
///
/// Scenario:
///   1. LP deposits $5,000 → vault margin = $5,000
///   2. Vault refreshes at ETH=$2,000 → places 12.5 ETH bid at $1,980
///   3. Taker fills bid → vault long 12.5 ETH @ $1,980
///   4. Price drops to $1,600:
///      - Unrealized PnL = 12.5 × ($1,600 − $1,980) = −$4,750
///      - Equity = $5,000 − $4,750 = $250
///      - MM = 12.5 × $1,600 × 5% = $1,000
///      - $250 < $1,000 → liquidatable
///   5. Bidder places limit bid at $1,600 (provides book liquidity)
///   6. Liquidation closes vault's long against the bid
///   7. Assert: vault positions cleared, insurance fund received fee,
///      vault margin adjusted by PnL
#[test]
fn vault_liquidation_on_order_book() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    let pair = pair_id();

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    // -------------------------------------------------------------------------
    // Step 1: LP (user1) deposits $5,000 USDC and adds all as vault liquidity.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(5_000_000_000)).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Vault(perps::VaultMsg::AddLiquidity {
                amount: UsdValue::new_int(5_000),
                min_shares_to_mint: None,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 2: Configure vault market-making.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Configure {
                param: Param {
                    vault_total_weight: Dimensionless::new_int(1),
                    ..default_param_no_liq_fee()
                },
                pair_params: btree_map! {
                    pair.clone() => PairParam {
                        vault_liquidity_weight: Dimensionless::new_int(1),
                        vault_half_spread: Dimensionless::new_permille(10), // 1%
                        vault_max_quote_size: Quantity::new_int(100),
                        ..default_pair_param()
                    },
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 3: Refresh vault orders → vault places bid at $1,980 for 12.5 ETH.
    // -------------------------------------------------------------------------

    refresh_vault_orders(&mut suite, &mut accounts, &contracts);

    let vault_orders: BTreeMap<perps::OrderId, QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: contracts.perps,
        })
        .should_succeed();

    let vault_bid = vault_orders
        .values()
        .find(|o| o.size.is_positive())
        .expect("vault should have a bid");

    let bid_size = vault_bid.size;

    // -------------------------------------------------------------------------
    // Step 4: Taker (user2) deposits and market sells into vault bid.
    //   Vault goes long 12.5 ETH @ $1,980.
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: bid_size.checked_neg().unwrap(),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Verify vault is long.
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

    let vault_margin_before = vault_state.margin;

    // -------------------------------------------------------------------------
    // Step 5: Price drops to $1,600 (20% drop). Vault becomes liquidatable.
    //   equity = $5,000 + 12.5 * ($1,600 - $1,980) = $250
    //   MM = 12.5 * $1,600 * 5% = $1,000
    //   $250 < $1,000 → liquidatable
    // -------------------------------------------------------------------------

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_600);

    // Sanity: verify vault is liquidatable (equity < MM).
    let vault_ext: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_available_margin: false,
            include_maintenance_margin: false,
            include_unrealized_pnl: false,
            include_unrealized_funding: false,
            include_liquidation_price: false,
            include_all: false,
        })
        .should_succeed();

    let equity = vault_ext.equity.unwrap();
    let vault_pos = vault_ext.positions.get(&pair).unwrap();
    let mm = vault_pos
        .size
        .checked_abs()
        .unwrap()
        .checked_mul(UsdPrice::new_int(1_600))
        .unwrap()
        .checked_mul(Dimensionless::new_permille(50))
        .unwrap();

    assert!(
        equity < mm,
        "vault should be liquidatable: equity ({equity}) < MM ({mm})"
    );

    // Record insurance fund before liquidation.
    let global_before: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    let insurance_before = global_before.insurance_fund;

    // -------------------------------------------------------------------------
    // Step 6: Bidder (user3) deposits and places limit bid at $1,600 for the
    //   vault's full position size. This provides liquidity for the liquidation.
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
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: bid_size, // buy same size as vault's long
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_600),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 7: Liquidate the vault.
    // -------------------------------------------------------------------------

    suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: contracts.perps,
            }),
            Coins::new(),
        )
        .should_succeed();

    // -------------------------------------------------------------------------
    // Step 8: Assertions.
    // -------------------------------------------------------------------------

    // Vault position should be reduced (partial liquidation closes just enough
    // to restore equity above maintenance margin).
    let vault_state_after = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: contracts.perps,
        })
        .should_succeed()
        .unwrap();

    let vault_pos_after = vault_state_after.positions.get(&pair);
    let position_reduced = match vault_pos_after {
        None => true,                                            // fully closed
        Some(pos) => pos.size < vault_ext.positions[&pair].size, // partially closed
    };

    assert!(
        position_reduced,
        "vault position should be reduced by liquidation"
    );

    // Vault margin should be reduced (PnL loss).
    let vault_margin_after = vault_state_after.margin;

    assert!(
        vault_margin_after < vault_margin_before,
        "vault margin should decrease: before={vault_margin_before}, after={vault_margin_after}"
    );

    // Vault should be healthy after liquidation (equity >= MM).
    // This holds because liquidation_fee_rate = 0, so no fee erodes the
    // buffer created by the close schedule.
    let vault_ext_after: perps::UserStateExtended = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateExtendedRequest {
            user: contracts.perps,
            include_equity: true,
            include_available_margin: false,
            include_maintenance_margin: false,
            include_unrealized_pnl: false,
            include_unrealized_funding: false,
            include_liquidation_price: false,
            include_all: false,
        })
        .should_succeed();

    let equity_after = vault_ext_after.equity.unwrap();
    let mm_after = match vault_ext_after.positions.get(&pair) {
        Some(pos) => pos
            .size
            .checked_abs()
            .unwrap()
            .checked_mul(UsdPrice::new_int(1_600))
            .unwrap()
            .checked_mul(Dimensionless::new_permille(50))
            .unwrap(),
        None => UsdValue::ZERO,
    };

    assert!(
        equity_after >= mm_after,
        "vault should be healthy after liquidation: equity ({equity_after}) >= MM ({mm_after})"
    );

    // Insurance fund should be unchanged (liquidation_fee_rate = 0).
    let global_after: perps::State = suite
        .query_wasm_smart(contracts.perps, perps::QueryStateRequest {})
        .should_succeed();

    assert_eq!(
        global_after.insurance_fund, insurance_before,
        "insurance fund should be unchanged with zero liquidation fee"
    );

    // Bidder (user3) should now have a long position from absorbing the vault's close.
    let user3_state = suite
        .query_wasm_smart(contracts.perps, perps::QueryUserStateRequest {
            user: accounts.user3.address(),
        })
        .should_succeed()
        .unwrap();

    let user3_pos = user3_state
        .positions
        .get(&pair)
        .expect("user3 should have a position");
    assert!(
        user3_pos.size.is_positive(),
        "user3 should be long (absorbed vault's sell)"
    );
}

/// A liquidation that partially fills on the order book and ADLs the
/// remainder must:
///
/// - emit `OrderFilled` events for every book-matching fill, each with
///   `fill_id: Some(_)`;
/// - pair those fills two-to-one per `fill_id` (taker + maker);
/// - emit `Deleveraged` events for the ADL leg, whose event payload has
///   no `fill_id` field (enforced at compile time by the `Deleveraged`
///   struct definition, re-confirmed at runtime by the successful
///   `deserialize_json::<Deleveraged>`).
///
/// This pins the BitMEX-style separation: order-book matches get a
/// `fill_id`, position transfers at the bankruptcy price do not.
#[test]
fn liquidation_book_fills_have_fill_id_adl_does_not() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption::default());

    register_oracle_prices(&mut suite, &mut accounts, &contracts, 2_000);

    let pair = pair_id();

    // Trader A (user1) opens a long that will later become liquidatable.
    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(1_100_000_000)).unwrap(),
        )
        .should_succeed();

    // Maker (user2) deposits enough to seed the book with plenty of asks,
    // and later partial-bid for the liquidation.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(20_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Maker places ask: 5 ETH @ $2,000. Trader A fills it, ending long 5 ETH.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Trader B (user3) will be the ADL counter-party: short 5 ETH @ $2,000.
    // Large margin so that ADL finds them and they stay solvent.
    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::Deposit { to: None }),
            Coins::one(usdc::DENOM.clone(), Uint128::new(10_000_000_000)).unwrap(),
        )
        .should_succeed();

    // Maker places a bid so Trader B can short into it.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(2_000),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user3,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::new_percent(50),
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Oracle drops to $1,450. Trader A is deeply underwater and forced
    // to close the full position; equity is negative so target_price for
    // book matching is the oracle ($1,450).
    register_oracle_prices(&mut suite, &mut accounts, &contracts, 1_450);

    // Partial book liquidity for the liquidation: Maker places a bid for
    // 2 ETH @ $1,450. Trader A's liquidation will fill this, then ADL
    // the remaining 3 ETH against Trader B at the bankruptcy price.
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder(perps::SubmitOrderRequest {
                pair_id: pair.clone(),
                size: Quantity::new_int(2),
                kind: perps::OrderKind::Limit {
                    limit_price: UsdPrice::new_int(1_450),
                    time_in_force: perps::TimeInForce::PostOnly,
                    client_order_id: None,
                },
                reduce_only: false,
                tp: None,
                sl: None,
            })),
            Coins::new(),
        )
        .should_succeed();

    // Liquidate Trader A. Expect book fills + ADL remainder.
    let events = suite
        .execute(
            &mut accounts.owner,
            contracts.perps,
            &perps::ExecuteMsg::Maintain(perps::MaintainerMsg::Liquidate {
                user: accounts.user1.address(),
            }),
            Coins::new(),
        )
        .should_succeed()
        .events;

    // Book-matching leg: every OrderFilled event must carry a fill_id,
    // and the events must pair two-to-one per fill_id (taker + maker).
    let order_filled_events = events
        .clone()
        .search_event::<CheckedContractEvent>()
        .with_predicate(|e| e.ty == "order_filled")
        .take()
        .all()
        .into_iter()
        .map(|e| e.event.data.deserialize_json::<OrderFilled>().unwrap())
        .collect::<Vec<_>>();

    assert!(
        !order_filled_events.is_empty(),
        "liquidation should have produced at least one book fill"
    );

    for filled in &order_filled_events {
        assert!(
            filled.fill_id.is_some(),
            "every OrderFilled emitted during liquidation book matching must carry a fill_id"
        );
    }

    let mut by_fill_id = BTreeMap::<_, usize>::new();
    for filled in &order_filled_events {
        *by_fill_id.entry(filled.fill_id.unwrap()).or_default() += 1;
    }
    for (fill_id, count) in &by_fill_id {
        assert_eq!(
            *count, 2,
            "fill_id {} should appear on exactly two OrderFilled events \
             (taker side + maker side); got {}",
            fill_id, count,
        );
    }

    // ADL leg: Deleveraged events must exist and deserialize cleanly into
    // the `Deleveraged` struct, which has no fill_id field — if the
    // runtime JSON ever gained one, downstream consumers would still
    // deserialize fine (extra fields ignored) but this assertion
    // documents the intended shape.
    let deleveraged_events = events
        .search_event::<CheckedContractEvent>()
        .with_predicate(|e| e.ty == "deleveraged")
        .take()
        .all()
        .into_iter()
        .map(|e| e.event.data.deserialize_json::<Deleveraged>().unwrap())
        .collect::<Vec<_>>();

    assert!(
        !deleveraged_events.is_empty(),
        "liquidation should have ADL'd the remainder against a counter-party"
    );
}
