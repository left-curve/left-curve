//! This file contains fixes to issues discovered during the Sherlock audit contest,
//! September 15-30, 2025.

use {
    dango_dex::liquidity_depth::get_bucket,
    dango_testing::{BridgeOp, TestOption, setup_test_naive},
    dango_types::{
        constants::{
            dango, eth,
            mock::{ONE, ONE_TENTH},
            usdc,
        },
        dex::{
            self, AmountOption, CreateOrderRequest, Direction, ExecuteMsg, Geometric,
            LiquidityDepth, OrderId, OrdersByUserResponse, PairParams, PairUpdate,
            PassiveLiquidity, Price, PriceOption, QueryPairRequest, SwapRoute, TimeInForce,
        },
        gateway::Remote,
        oracle::{self, PriceSource},
    },
    grug::{
        Bounded, Coin, Coins, Denom, Inner, NonZero, Number, NumberConst, QuerierExt, ResultExt,
        Timestamp, Udec128, Udec128_6, Uint128, UniqueVec, btree_map, btree_set, coins,
    },
    grug_types::Addressable,
    hyperlane_types::constants::ethereum,
    rand::Rng,
    std::{collections::HashMap, str::FromStr},
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
                    precision: 0,
                    timestamp: Timestamp::from_nanos(u128::MAX), // use max timestamp so the oracle price isn't rejected for being too old
                },
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
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
                    }),
                    bucket_sizes: btree_set! {
                        NonZero::new_unchecked(ONE_TENTH),
                        NonZero::new_unchecked(ONE),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    min_order_size: Uint128::ZERO,
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
            },
            coins! {
                dango::DENOM.clone() => 100_000,
            },
        )
        .should_fail_with_error("lp mint amount must be non-zero");
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
