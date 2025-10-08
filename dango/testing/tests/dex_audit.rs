//! This file contains fixes to issues discovered during the Sherlock audit contest,
//! September 15-30, 2025.

use {
    dango_testing::setup_test_naive,
    dango_types::{
        constants::{ONE, ONE_TENTH, dango, eth, usdc},
        dex::{
            self, AmountOption, CreateOrderRequest, Direction, Geometric, LiquidityDepth, OrderId,
            OrdersByUserResponse, PairParams, PairUpdate, PassiveLiquidity, Price, PriceOption,
            SwapRoute, TimeInForce,
        },
        oracle::{self, PriceSource},
    },
    grug::{
        Bounded, Coin, Coins, Denom, NonZero, NumberConst, QuerierExt, ResultExt, Timestamp,
        Udec128, Udec128_6, Uint128, UniqueVec, btree_map, btree_set, coins,
    },
    std::str::FromStr,
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
