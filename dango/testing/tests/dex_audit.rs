//! This file contains fixes to issues discovered during the Sherlock audit contest,
//! September 15-30, 2025.

use {
    dango_dex::liquidity_depth::get_bucket,
    dango_genesis::{DexOption, GenesisOption, OracleOption},
    dango_testing::{
        BridgeOp, Preset, TestOption, setup_test_naive, setup_test_naive_with_custom_genesis,
    },
    dango_types::{
        constants::{
            dango, eth,
            mock::{ONE, ONE_TENTH},
            usdc,
        },
        dex::{
            self, AmountOption, AvellanedaStoikovParams, CreateOrderRequest, Direction, ExecuteMsg,
            Geometric, LiquidityDepth, OrderId, OrdersByPairResponse, OrdersByUserResponse,
            PairParams, PairUpdate, PassiveLiquidity, Price, PriceOption, QueryPairRequest,
            SwapRoute, TimeInForce,
        },
        gateway::Remote,
        oracle::{self, PriceSource},
    },
    grug::{
        BalanceChange, Bounded, Coin, CoinPair, Coins, Dec, Denom, Duration, Inner, NonZero,
        Number, NumberConst, QuerierExt, ResultExt, Timestamp, Udec128, Udec128_6, Uint128,
        UniqueVec, btree_map, btree_set, coins,
    },
    grug_types::Addressable,
    hyperlane_types::constants::ethereum,
    rand::Rng,
    std::{
        collections::{BTreeSet, HashMap},
        str::FromStr,
    },
};

/// Prior to the fix, liquidity depth from orders placed by the passive pool wasn't
/// decreased if the order is filled. Only liquidity from user orders were properly
/// decreased.
#[test]
fn liquidity_depth_from_passive_pool_decreased_properly_when_order_filled() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Supply oracle prices. DANGO = $200, USDC = $1.
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::new(200),
                    precision: dango::DECIMAL as u8,  // Should match token decimals
                    timestamp: Timestamp::from_nanos(u128::MAX), // use max timestamp so the oracle price isn't rejected for being too old
                },
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: usdc::DECIMAL as u8,  // Should match token decimals
                    timestamp: Timestamp::from_nanos(u128::MAX),
                }
            }),
            Coins::new(),
        )
        .should_succeed();

    // Configure DANGO-USD pool type to geometric with spacing 1. It's easier to
    // work with than xyk.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                    pool_type: PassiveLiquidity::Geometric(Geometric {
                        spacing: Udec128::ONE,
                        ratio: Bounded::new_unchecked(Udec128::ONE),
                        limit: 1,
                        avellaneda_stoikov_params: AvellanedaStoikovParams {
                            // For DANGO/USDC with oracle_price = 200, swap_fee_rate = 0.003:
                            // We want half_spread/reservation_price = 0.003
                            // half_spread = ln(1 + gamma), reservation_price = 200
                            // So ln(1 + gamma) = 0.003 * 200 = 0.6
                            // gamma = e^0.6 - 1 ≈ 0.8221
                            gamma: Dec::from_str("0.8221").unwrap(),
                            time_horizon: Duration::from_seconds(0),
                            k: Dec::ONE,
                            lambda: Dec::ZERO,
                        },
                    }),
                    bucket_sizes: btree_set! {
                        NonZero::new_unchecked(ONE_TENTH),
                        NonZero::new_unchecked(ONE),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    min_order_size_quote: Uint128::ZERO,
                    min_order_size_base: Uint128::ZERO,
                },
            }])),
            Coins::new(),
        )
        .should_succeed();

    // Add liquidity to the DANGO-USDC pool: 10 dango, 2,000 USDC (2_000_000_000 uusdc)
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            coins! {
                dango::DENOM.clone() => 10,
                usdc::DENOM.clone() => 2_000_000_000,
            },
        )
        .should_succeed();

    // Query the liquidity depth before placing any order. There should be an
    // order on each side of the oracle price (200 +/- 0.3%) with 100% of the
    // liquidity.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryLiquidityDepthRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            bucket_size: ONE_TENTH,
            limit: Some(10),
        })
        .should_succeed_and_equal(dex::LiquidityDepthResponse {
            bid_depth: Some(vec![(
                Price::new(199_400_000), // 200 * (1 - 0.3%), considering 6 decimal difference between dango and usdc
                LiquidityDepth {
                    depth_base: Udec128_6::new(10), // floor(available quote liquidity / price) = floor(2_000_000_000 / 199_400_000) = 1
                    depth_quote: Udec128_6::new(1_994_000_000), // order size in base * price = 10 * 199_400_000 = 1_994_000_000
                },
            )]),
            ask_depth: Some(vec![(
                Price::new(200_600_000), // 200 * (1 + 0.3%)
                LiquidityDepth {
                    depth_base: Udec128_6::new(10), // available base liquidity = 10
                    depth_quote: Udec128_6::new(2_006_000_000), // base liquidity * price = 10 * 200_600_000 = 2_006_000_000
                },
            )]),
        });

    // Query the orders placed by the DEX itself.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByUserRequest {
            user: contracts.dex,
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map! {
            OrderId::from(!1) => OrdersByUserResponse {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                price: Price::new(199_400_000),
                amount: Uint128::new(10),
                remaining: Udec128_6::new(10),
            },
            OrderId::from(2) => OrdersByUserResponse {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Ask,
                price: Price::new(200_600_000),
                amount: Uint128::new(10),
                remaining: Udec128_6::new(10),
            },
        });

    // Now place an order that consumes some of the ask side liquidity.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    price: PriceOption::Limit(NonZero::new_unchecked(Price::new(200_600_000))),
                    amount: AmountOption::Bid {
                        quote: NonZero::new_unchecked(Uint128::new(200_600_000 * 3)), // should create an order with base amount 3
                    },
                    time_in_force: TimeInForce::GoodTilCanceled,
                }],
                cancels: None,
            },
            coins! { usdc::DENOM.clone() => 200_600_000 * 3 },
        )
        .should_succeed();

    // The ask side liquidity should be reduced from 10 base to 10 - 3 = 7.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryLiquidityDepthRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            bucket_size: ONE_TENTH,
            limit: Some(10),
        })
        .should_succeed_and_equal(dex::LiquidityDepthResponse {
            // bid side is unchanged
            bid_depth: Some(vec![(Price::new(199_400_000), LiquidityDepth {
                depth_base: Udec128_6::new(10),
                depth_quote: Udec128_6::new(1_994_000_000),
            })]),
            ask_depth: Some(vec![(Price::new(200_600_000), LiquidityDepth {
                depth_base: Udec128_6::new(7),                // decreased!
                depth_quote: Udec128_6::new(200_600_000 * 7), // decreased!
            })]),
        });

    // Check the orders as well.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByUserRequest {
            user: contracts.dex,
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map! {
            OrderId::from(!4) => OrdersByUserResponse {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                price: Price::new(199_400_000),
                amount: Uint128::new(10),
                remaining: Udec128_6::new(10),
            },
            OrderId::from(5) => OrdersByUserResponse {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Ask,
                price: Price::new(200_600_000),
                amount: Uint128::new(10),
                remaining: Udec128_6::new(7), // decreased!
            },
        });
}

#[test]
fn issue_6_cannot_mint_zero_lp_tokens() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Provide liquidity with zero amount of one side
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            coins! {
                dango::DENOM.clone() => 100_000,
            },
        )
        .should_fail_with_error("LP token mint amount is less than `MINIMUM_LIQUIDITY`");
}

#[test]
fn issue_10_rounding_up_in_xyk_swap_exact_amount_out() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Provide liquidity with owner account
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            coins! {
                eth::DENOM.clone() => 100_000,
                usdc::DENOM.clone() => 100_000,
            },
        )
        .should_succeed();

    // Record user balance
    suite.balances().record(&accounts.user1);

    // Swap exact amount out
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountOut {
                route: SwapRoute::new_unchecked(
                    UniqueVec::new(vec![dex::PairId {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                    }])
                    .unwrap(),
                ),
                output: NonZero::new_unchecked(Coin::new(usdc::DENOM.clone(), 100).unwrap()),
            },
            coins! {
                eth::DENOM.clone() => 103,
            },
        )
        .should_succeed();

    // Assert that the user's balance has changed correctly
    suite.balances().should_change(&accounts.user1, btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(103),
        usdc::DENOM.clone() => BalanceChange::Increased(100),
    });
}

#[test]
fn issue_30_liquidity_operations_are_not_allowed_when_dex_is_paused() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::SetPaused(true)),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            Coins::new(),
        )
        .should_fail_with_error("can't provide liquidity when trading is paused");

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::WithdrawLiquidity {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            Coins::new(),
        )
        .should_fail_with_error("can't withdraw liquidity when trading is paused");

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountIn {
                route: SwapRoute::new_unchecked(UniqueVec::new(vec![]).unwrap()),
                minimum_output: None,
            },
            Coins::new(),
        )
        .should_fail_with_error("can't swap when trading is paused");

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountOut {
                route: SwapRoute::new_unchecked(UniqueVec::new(vec![]).unwrap()),
                output: NonZero::new_unchecked(Coin::new(eth::DENOM.clone(), 100).unwrap()),
            },
            Coins::new(),
        )
        .should_fail_with_error("can't swap when trading is paused");

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![],
                cancels: None,
            },
            Coins::new(),
        )
        .should_fail_with_error("can't update orders when trading is paused");
}

/// Prior to the fix, the depth quote amount was subject to rounding errors when
/// a limit order was partially filled and the error kept increasing over time.
#[test]
fn issue_156_depth_quote_rounding_error() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption {
        bridge_ops: |accounts| {
            vec![
                BridgeOp {
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::USDC_WARP,
                    },
                    amount: Uint128::new(1_000_000_000_000_000_000),
                    recipient: accounts.user1.address(),
                },
                BridgeOp {
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::WETH_WARP,
                    },
                    amount: Uint128::new(1_000_000_000_000_000_000),
                    recipient: accounts.user1.address(),
                },
            ]
        },
        ..Default::default()
    });

    let balance = suite.query_balances(&accounts.user1.address()).unwrap();

    let base_denom = eth::DENOM.clone();
    let quote_denom = usdc::DENOM.clone();

    let pair_info = suite
        .query_wasm_smart(contracts.dex, QueryPairRequest {
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
        })
        .unwrap();

    let denominator = Uint128::new(10_u128.pow(eth::DECIMAL - usdc::DECIMAL));

    // Open a Bid order at really small price and a Ask order at a really large price
    // in order to non delete all depth from contract.
    let bid_order = CreateOrderRequest {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        price: PriceOption::Limit(NonZero::new_unchecked(Price::raw(Uint128::new(1)))), /* 1e-18 USDC per wei */
        amount: AmountOption::Bid {
            quote: NonZero::new_unchecked(Uint128::new(1_000_000)), // 1 USDC
        },
        time_in_force: TimeInForce::GoodTilCanceled,
    };

    let ask_order = CreateOrderRequest {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        price: PriceOption::Limit(NonZero::new_unchecked(Price::new(10u128.pow(10)))), /* 1e6 USDC per wei */
        amount: AmountOption::Ask {
            base: NonZero::new_unchecked(Uint128::new(1_000_000_000_000)), // 1 ETH
        },
        time_in_force: TimeInForce::GoodTilCanceled,
    };

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &ExecuteMsg::BatchUpdateOrders {
                creates: vec![bid_order, ask_order],
                cancels: None,
            },
            coins!(
                base_denom.clone() => Uint128::new(1_000_000_000_000),
                quote_denom.clone() => Uint128::new(1_000_000),
            ),
        )
        .should_succeed();

    // Create some matching orders:
    // open 1 ask limit order with size x and price p;
    // open 2 buy limit order with size x/2 and price p.
    let min_price = Price::checked_from_ratio(500, denominator).unwrap();
    let max_price = Price::checked_from_ratio(4_000, denominator).unwrap();

    for _ in 0..100 {
        let mut orders = vec![];

        let price = Price::raw(Uint128::new(
            rand::thread_rng().gen_range(min_price.0.into_inner()..max_price.0.into_inner()),
        ));

        let sell_amount = Uint128::new(
            rand::thread_rng()
                .gen_range(1_000u128..=balance.amount_of(&base_denom).into_inner().div_ceil(2)),
        );

        // buy_amount = sell_amount * price / 2
        let buy_amount = Udec128_6::checked_from_ratio(sell_amount, Uint128::new(2))
            .unwrap()
            .checked_mul(price)
            .unwrap()
            .into_int();

        let sell_order = CreateOrderRequest {
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            price: PriceOption::Limit(NonZero::new_unchecked(price)),
            amount: AmountOption::Ask {
                base: NonZero::new_unchecked(sell_amount),
            },
            time_in_force: TimeInForce::GoodTilCanceled,
        };

        let buy_order = CreateOrderRequest {
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            price: PriceOption::Limit(NonZero::new_unchecked(price)),
            amount: AmountOption::Bid {
                quote: NonZero::new_unchecked(buy_amount),
            },
            time_in_force: TimeInForce::GoodTilCanceled,
        };

        orders.push(sell_order);
        orders.push(buy_order.clone());
        orders.push(buy_order);

        suite
            .execute(
                &mut accounts.user1,
                contracts.dex,
                &ExecuteMsg::BatchUpdateOrders {
                    creates: orders,
                    cancels: None,
                },
                coins!(
                    base_denom.clone() => sell_amount,
                    quote_denom.clone() => buy_amount * Uint128::new(2),
                ),
            )
            .should_succeed();

        // Check the liquidity depth.

        // Query the open orders for this pair.
        let open_orders = suite
            .query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
                start_after: None,
                limit: Some(u32::MAX),
            })
            .unwrap();

        // Ensure open orders are not empty.
        assert!(!open_orders.is_empty());

        for bucket_size in &pair_info.bucket_sizes {
            let mut bid_amounts = HashMap::new();
            let mut ask_amounts = HashMap::new();

            for order in open_orders.values() {
                let amounts = match order.direction {
                    Direction::Bid => &mut bid_amounts,
                    Direction::Ask => &mut ask_amounts,
                };

                let bucket =
                    get_bucket(bucket_size.into_inner(), order.direction, order.price).unwrap();

                let entry = amounts
                    .entry(bucket)
                    .or_insert((Udec128_6::ZERO, Udec128_6::ZERO));

                entry.0.checked_add_assign(order.remaining).unwrap();
                entry
                    .1
                    .checked_add_assign(order.remaining.checked_mul(order.price).unwrap())
                    .unwrap();
            }

            // Retrieve liquidity from the contract.
            let depth = suite
                .query_wasm_smart(contracts.dex, dex::QueryLiquidityDepthRequest {
                    base_denom: base_denom.clone(),
                    quote_denom: quote_denom.clone(),
                    bucket_size: bucket_size.into_inner(),
                    limit: None,
                })
                .unwrap();

            let bid_depth = depth.bid_depth.unwrap();

            for (price, liquidity) in bid_depth {
                let (expected_base, expected_quote) = bid_amounts
                    .get(&price)
                    .unwrap_or(&(Udec128_6::ZERO, Udec128_6::ZERO));

                assert_eq!(
                    &liquidity.depth_base, expected_base,
                    "mismatched bid depth base for bucket size {bucket_size} at price {price}"
                );
                assert_eq!(
                    &liquidity.depth_quote, expected_quote,
                    "mismatched bid depth quote for bucket size {bucket_size} at price {price}"
                );
            }

            let ask_depth = depth.ask_depth.unwrap();
            for (price, liquidity) in ask_depth {
                let (expected_base, expected_quote) = ask_amounts
                    .get(&price)
                    .unwrap_or(&(Udec128_6::ZERO, Udec128_6::ZERO));

                assert_eq!(
                    &liquidity.depth_base, expected_base,
                    "mismatched ask depth base for bucket size {bucket_size} at price {price}"
                );
                assert_eq!(
                    &liquidity.depth_quote, expected_quote,
                    "mismatched ask depth quote for bucket size {bucket_size} at price {price}"
                );
            }
        }
    }
}

/// In `cancel_all_orders`, we refund all orders. However, actually, passive
/// orders don't need to be refunded. Refunding it leads to error because the
/// DEX contract doesn't implement the `receive` entry point.
#[test]
fn issue_194_cancel_all_orders_works_properly_with_passive_orders() {
    // ------------------------------- 1. Setup --------------------------------

    // Set up DANGO-USDC pair with geometric pool and oracle feed.
    let (mut suite, mut accounts, _, contracts, _) =
        setup_test_naive_with_custom_genesis(Default::default(), GenesisOption {
            dex: DexOption {
                pairs: vec![PairUpdate {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Geometric(Geometric {
                            spacing: Udec128::new_percent(1),
                            ratio: Bounded::new_unchecked(Udec128::new(1)),
                            limit: 1,
                            avellaneda_stoikov_params: AvellanedaStoikovParams {
                                // For DANGO/USDC with oracle_price = 200, swap_fee_rate = 0.003:
                                // We want half_spread/reservation_price = 0.003
                                // half_spread = ln(1 + gamma), reservation_price = 200
                                // So ln(1 + gamma) = 0.003 * 200 = 0.6
                                // gamma = e^0.6 - 1 ≈ 0.8221
                                gamma: Dec::from_str("0.8221").unwrap(),
                                time_horizon: Duration::from_seconds(0),
                                k: Dec::ONE,
                                lambda: Dec::ZERO,
                            },
                        }),
                        bucket_sizes: BTreeSet::new(),
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
                    },
                }],
            },
            oracle: OracleOption {
                pyth_price_sources: btree_map! {
                    dango::DENOM.clone() => PriceSource::Fixed {
                        humanized_price: Udec128::new(200),
                        precision: 0,
                        timestamp: Timestamp::from_nanos(u128::MAX),
                    },
                    usdc::DENOM.clone() => PriceSource::Fixed {
                        humanized_price: Udec128::new(1),
                        precision: 0,
                        timestamp: Timestamp::from_nanos(u128::MAX),
                    },
                },
                ..Preset::preset_test()
            },
            ..Preset::preset_test()
        });

    // Provide some liquidity to the pool.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: None,
            },
            coins! {
                dango::DENOM.clone() => 5,
                usdc::DENOM.clone() => 1000,
            },
        )
        .should_succeed();

    // For realism, also create some user orders.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::new(195)),
                        NonZero::new_unchecked(Uint128::new(195)),
                    ),
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::new(205)),
                        NonZero::new_unchecked(Uint128::new(1)),
                    ),
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => 1,
                usdc::DENOM.clone() => 195,
            },
        )
        .should_succeed();

    // Pause trading. We want to ensure that forced order cancelations works
    // when trading is paused (while all other operations that affect liquidity
    // are disabled).
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::SetPaused(true)),
            Coins::new(),
        )
        .should_succeed();

    // -------------- 2. Ensure contract state before cancelation --------------

    // Check the DEX contract's balances.
    // Should equal the liquidity provided + user orders.
    suite
        .query_balance(&contracts.dex, dango::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(5 + 1));
    suite
        .query_balance(&contracts.dex, usdc::DENOM.clone())
        .should_succeed_and_equal(Uint128::new(1000 + 195));

    // Check the pool's reserves. Should equal the liquidity provided.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryReserveRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(CoinPair::new_unchecked(
            Coin {
                denom: usdc::DENOM.clone(),
                amount: Uint128::new(1000),
            },
            Coin {
                denom: dango::DENOM.clone(),
                amount: Uint128::new(5),
            },
        ));

    // Check orders. Should be two user orders, two passive orders.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map! {
            // Two user orders:
            OrderId::new(!3) => OrdersByPairResponse {
                user: accounts.user1.address(),
                direction: Direction::Bid,
                price: Price::new(195),
                amount: Uint128::new(1),
                remaining: Udec128_6::new(1),
            },
            OrderId::new(4) => OrdersByPairResponse {
                user: accounts.user1.address(),
                direction: Direction::Ask,
                price: Price::new(205),
                amount: Uint128::new(1),
                remaining: Udec128_6::new(1),
            },
            // Two passive orders:
            OrderId::new(!5) => OrdersByPairResponse {
                user: contracts.dex,
                direction: Direction::Bid,
                price: Price::from_str("199.4").unwrap(), // = oracle_price * (1 - swap_fee_rate) = 200 * (1 - 0.003) = 199.4
                amount: Uint128::new(5), // = floor(quote_reserve / price) = floor(1000 / 199.4) = 5
                remaining: Udec128_6::new(5),
            },
            OrderId::new(6) => OrdersByPairResponse {
                user: contracts.dex,
                direction: Direction::Ask,
                price: Price::from_str("200.6").unwrap(), // = oracle_price * (1 + swap_fee_rate) = 200 * (1 + 0.003) = 200.6
                amount: Uint128::new(5), // = base_reserve = 5
                remaining: Udec128_6::new(5),
            },
        });

    // --------------- 3. Perform the forced order cancelations ----------------

    suite
        .balances()
        .record_many([&contracts.dex, &accounts.user1.address()]);

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::Reset {}),
            Coins::new(),
        )
        .should_succeed(); // Prior to the fix, this errors.

    // --------------- 4. Check contract state after cancelation ---------------

    // Ensure token balances and in/outflows.
    suite.balances().should_change(&contracts.dex, btree_map! {
        dango::DENOM.clone() => BalanceChange::Decreased(1),
        usdc::DENOM.clone() => BalanceChange::Decreased(195),
    });
    suite.balances().should_change(&accounts.user1, btree_map! {
        dango::DENOM.clone() => BalanceChange::Increased(1),
        usdc::DENOM.clone() => BalanceChange::Increased(195),
    });

    // Ensure reserves are unchanged.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryReserveRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(CoinPair::new_unchecked(
            Coin {
                denom: usdc::DENOM.clone(),
                amount: Uint128::new(1000),
            },
            Coin {
                denom: dango::DENOM.clone(),
                amount: Uint128::new(5),
            },
        ));

    // Ensure orders are emptied.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and(|orders| orders.is_empty());
}

#[test]
fn issue_233_minimum_order_size_cannot_be_circumvented_for_ask_orders() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Create an ask order of one base unit with a price equal to the minimum order size.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(50)),
                    NonZero::new_unchecked(Uint128::new(1)),
                )],
                cancels: None,
            },
            coins! { dango::DENOM.clone() => 1 },
        )
        .should_fail();
}
