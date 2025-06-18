use {
    dango_genesis::Contracts,
    dango_oracle::OracleQuerier,
    dango_testing::{BridgeOp, TestAccounts, TestOption, TestSuite, setup_test_naive},
    dango_types::{
        DangoQuerier,
        constants::{dango, eth, sol, usdc},
        dex::{
            self, CreateLimitOrderRequest, Direction, OrderId, OrderResponse, PairId, PairParams,
            PairUpdate, PassiveLiquidity, ReservesResponse, SwapRoute,
        },
        gateway::Remote,
    },
    grug::{
        Addressable, Bounded, Coin, Coins, Dec128, Denom, Inner, MaxLength, MultiplyFraction,
        NonZero, Number, NumberConst, QuerierExt, ResultExt, Signed, Udec128, Uint128, UniqueVec,
        btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    hyperlane_types::constants::{ethereum, solana},
    proptest::{prelude::*, proptest, sample::select},
    std::{
        collections::{BTreeMap, HashMap},
        fmt::Display,
        ops::{Div, Sub},
        str::FromStr,
    },
};

/// Calculates the relative difference between two values.
fn relative_difference<T>(a: T, b: T) -> T
where
    T: NumberConst + Number + PartialOrd + Sub<Output = T> + Div<Output = T>,
{
    // Handle the case where both numbers are zero
    if a == T::ZERO && b == T::ZERO {
        return T::ZERO;
    }

    // Calculate absolute difference
    let abs_diff = if a > b {
        a - b
    } else {
        b - a
    };

    // Calculate the larger of the two values for relative comparison
    let larger = if a > b {
        a
    } else {
        b
    };

    // Calculate relative difference
    abs_diff / larger
}

/// Asserts that two values are approximately equal within a specified
/// relative difference.
fn assert_approx_eq<T>(a: T, b: T, max_rel_diff: &str) -> Result<(), TestCaseError>
where
    T: NumberConst + Number + PartialOrd + Sub<Output = T> + Div<Output = T> + Display,
{
    let rel_diff = Udec128::from_str(relative_difference(a, b).to_string().as_str()).unwrap();

    prop_assert!(
        rel_diff <= Udec128::from_str(max_rel_diff).unwrap(),
        "assertion failed: values are not approximately equal\n  left: {}\n right: {}\n  max_rel_diff: {}\n  actual_rel_diff: {}",
        a,
        b,
        max_rel_diff,
        rel_diff
    );

    Ok(())
}

/// Helper function to register a fixed price for a denom
fn register_fixed_price(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
    denom: Denom,
    humanized_price: Udec128,
    precision: u8,
) {
    // Register price source
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                denom => dango_types::oracle::PriceSource::Fixed {
                    humanized_price,
                    precision,
                    timestamp: 0,
                }
            }),
            Coins::default(),
        )
        .should_succeed();
}

/// Query the TVL of a list of coins.
fn tvl_of_coins(coins: &Coins, oracle_querier: &mut OracleQuerier) -> Udec128 {
    let mut tvl = Udec128::ZERO;
    for coin in coins {
        let price = oracle_querier.query_price(coin.denom, None).unwrap();
        tvl += price.value_of_unit_amount(*coin.amount).unwrap();
    }
    tvl
}

/// Query the TVL of a list of orders.
fn tvl_of_orders(
    orders: &BTreeMap<OrderId, OrderResponse>,
    oracle_querier: &mut OracleQuerier,
) -> Udec128 {
    let mut tvl = Udec128::ZERO;
    for order in orders.values() {
        let denom = if order.direction == Direction::Bid {
            &order.quote_denom
        } else {
            &order.base_denom
        };
        let price = oracle_querier.query_price(denom, None).unwrap();
        tvl += price.value_of_unit_amount(order.amount).unwrap();
    }
    tvl
}

/// Query the TVL of a list of reserves.
fn tvl_of_reserves(reserves: &[ReservesResponse], oracle_querier: &mut OracleQuerier) -> Udec128 {
    let mut coins = Coins::new();
    for reserve in reserves {
        coins
            .insert_many(reserve.reserve.clone().into_iter())
            .unwrap();
    }
    tvl_of_coins(&coins, oracle_querier)
}

/// A list of actions that can be performed on the dex contract.
#[derive(Debug, Clone)]
pub enum DexAction {
    CreateLimitOrder {
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
        price: Udec128,
    },
    ProvideLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
        funds: Coins,
    },
    // WithdrawLiquidity {
    //     base_denom: Denom,
    //     quote_denom: Denom,
    //     funds: Coins,
    // },
    SwapExactAmountIn {
        route: SwapRoute,
        input: Coin,
    },
    SwapExactAmountOut {
        route: SwapRoute,
        output: Coin,
        funds: Coins,
    },
}

impl DexAction {
    fn execute(
        &self,
        suite: &mut TestSuite<NaiveProposalPreparer>,
        accounts: &mut TestAccounts,
        contracts: &Contracts,
    ) {
        match self {
            DexAction::CreateLimitOrder {
                base_denom,
                quote_denom,
                direction,
                amount,
                price,
            } => {
                let deposit = match direction {
                    Direction::Bid => Coin {
                        denom: quote_denom.clone(),
                        amount: amount.checked_mul_dec_ceil(*price).unwrap(),
                    },
                    Direction::Ask => Coin {
                        denom: base_denom.clone(),
                        amount: *amount,
                    },
                };
                suite
                    .execute(
                        &mut accounts.user1,
                        contracts.dex,
                        &dex::ExecuteMsg::BatchUpdateOrders {
                            creates_market: vec![],
                            creates_limit: vec![CreateLimitOrderRequest {
                                base_denom: base_denom.clone(),
                                quote_denom: quote_denom.clone(),
                                direction: *direction,
                                amount: *amount,
                                price: *price,
                            }],
                            cancels: None,
                        },
                        Coins::one(deposit.denom, deposit.amount).unwrap(),
                    )
                    .should_succeed();
            },
            DexAction::ProvideLiquidity {
                base_denom,
                quote_denom,
                funds,
            } => {
                suite
                    .execute(
                        &mut accounts.user1,
                        contracts.dex,
                        &dex::ExecuteMsg::ProvideLiquidity {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                        },
                        funds.clone(),
                    )
                    .should_succeed();
            },
            DexAction::SwapExactAmountIn { route, input } => {
                suite
                    .execute(
                        &mut accounts.user1,
                        contracts.dex,
                        &dex::ExecuteMsg::SwapExactAmountIn {
                            route: route.clone(),
                            minimum_output: None,
                        },
                        input.clone(),
                    )
                    .should(|tx_outcome| {
                        if tx_outcome.result.is_err() {
                            tx_outcome.should_fail_with_error("insufficient liquidity");
                        } else {
                            tx_outcome.should_succeed();
                        }
                        true
                    });
            },
            DexAction::SwapExactAmountOut {
                route,
                output,
                funds,
            } => {
                suite
                    .execute(
                        &mut accounts.user1,
                        contracts.dex,
                        &dex::ExecuteMsg::SwapExactAmountOut {
                            route: route.clone(),
                            output: NonZero::new(output.clone()).unwrap(),
                        },
                        funds.clone(),
                    )
                    .should(|tx_outcome| {
                        if tx_outcome.result.is_err() {
                            tx_outcome.should_fail_with_error("insufficient liquidity");
                        } else {
                            tx_outcome.should_succeed();
                        }
                        true
                    });
            },
        };
    }
}

fn denoms() -> Vec<Denom> {
    vec![
        usdc::DENOM.clone(),
        dango::DENOM.clone(),
        sol::DENOM.clone(),
        eth::DENOM.clone(),
    ]
}

/// Fixed set of pair ids
fn pair_ids() -> Vec<PairId> {
    vec![
        PairId {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        },
        PairId {
            base_denom: sol::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        },
        PairId {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        },
    ]
}

/// Proptest strategy for generating a pair id
fn pair_id() -> impl Strategy<Value = PairId> {
    select(pair_ids())
}

/// Proptest strategy for generating an order direction
fn direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Bid), Just(Direction::Ask)]
}

pub const MAX_AMOUNT: Uint128 = Uint128::new(1_000_000_000u128);

/// Proptest strategy for generating an amount between 1 and 1 billion microunits
fn amount() -> impl Strategy<Value = Uint128> {
    (1u128..1_000_000_000u128).prop_map(Uint128::new)
}

/// Proptest strategy for generating a price as [-3, 3] permille from 1.0
fn price() -> impl Strategy<Value = Udec128> {
    (-3i128..3i128).prop_map(|price_diff| {
        (Dec128::ONE - Dec128::new_permille(price_diff))
            .checked_into_unsigned()
            .unwrap()
    })
}

/// Proptest strategy for generating a SwapRoute
///
/// A SwapRoute can contain 1 or 2 unique pairs. For 2-pair routes, they must be chainable
/// (the quote denom of the first pair should match the base or quote denom of the second pair).
/// Since all our current pairs use USDC as the quote denom, any two different pairs can be chained.
fn swap_route() -> impl Strategy<Value = SwapRoute> {
    prop_oneof![
        // Single pair route
        pair_id().prop_map(|pair| {
            let unique_vec = UniqueVec::new(vec![pair]).unwrap();
            MaxLength::new(unique_vec).unwrap()
        }),
        // Two pair route - select from all valid combinations of different pairs
        // Since all pairs use USDC as quote, any two different pairs are chainable
        select(vec![(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)]).prop_map(|(i, j)| {
            let pairs = pair_ids();
            let unique_vec = UniqueVec::new(vec![pairs[i].clone(), pairs[j].clone()]).unwrap();
            MaxLength::new(unique_vec).unwrap()
        })
    ]
}

fn dex_action() -> impl Strategy<Value = DexAction> {
    prop_oneof![
        (price(), pair_id(), direction(), amount()).prop_map(
            move |(price, pair_id, direction, amount)| {
                DexAction::CreateLimitOrder {
                    base_denom: pair_id.base_denom,
                    quote_denom: pair_id.quote_denom,
                    direction,
                    amount,
                    price,
                }
            }
        ),
        (pair_id(), amount(), amount()).prop_map(move |(pair_id, amount1, amount2)| {
            DexAction::ProvideLiquidity {
                base_denom: pair_id.base_denom.clone(),
                quote_denom: pair_id.quote_denom.clone(),
                funds: coins! {
                    pair_id.base_denom => amount1,
                    pair_id.quote_denom => amount2,
                },
            }
        }),
        (swap_route(), amount()).prop_flat_map(move |(route, amount)| {
            // Use the first pair in the route to determine the input denom
            let first_pair = route.inner().inner()[0].clone();
            let first_pair_denoms = vec![
                first_pair.base_denom.clone(),
                first_pair.quote_denom.clone(),
            ];

            // If the route only has one pair, select either the base or quote denom as the input denom.
            // Otherwise, we have to select the base denom, since the swap has to go from base denom to USDC to next pair base denom,
            let available_denoms = if route.inner().inner().len() == 1 {
                first_pair_denoms
            } else {
                vec![first_pair.base_denom.clone()]
            };
            select(available_denoms).prop_map(move |denom| DexAction::SwapExactAmountIn {
                route: route.clone(),
                input: Coin::new(denom, amount).unwrap(),
            })
        }),
        (swap_route(), amount()).prop_flat_map(move |(route, amount)| {
            // Use the last pair in the route to determine the output denom
            let last_pair = route.inner().inner().last().unwrap().clone();
            let last_pair_denoms =
                vec![last_pair.base_denom.clone(), last_pair.quote_denom.clone()];

            // If the route only has one pair, select either the base or quote denom as the output denom.
            // Otherwise, we have to select the base denom, since the swap has to go from base denom to USDC to next pair base denom,
            let available_denoms = if route.inner().inner().len() == 1 {
                last_pair_denoms
            } else {
                vec![last_pair.base_denom.clone()]
            };
            select(available_denoms).prop_map(move |output_denom| {
                // If the route only has one pair, select the other denom as the input denom.
                // Otherwise we have to select the first pair's base denom as the input denom.
                let input_denom = if route.inner().inner().len() == 1 {
                    if output_denom == last_pair.base_denom {
                        last_pair.quote_denom.clone()
                    } else {
                        last_pair.base_denom.clone()
                    }
                } else {
                    route.inner().inner()[0].base_denom.clone()
                };
                DexAction::SwapExactAmountOut {
                    route: route.clone(),
                    output: Coin::new(output_denom, amount).unwrap(),
                    funds: Coins::one(input_denom, MAX_AMOUNT * Uint128::new(10_000)).unwrap(),
                }
            })
        })
    ]
}

fn dex_actions(min_size: usize, max_size: usize) -> impl Strategy<Value = Vec<DexAction>> {
    (min_size..=max_size)
        .prop_flat_map(move |size| {
            // Generate pairs of (action, two amounts for potential liquidity provision)
            (1..=size)
                .collect::<Vec<_>>()
                .into_iter()
                .map(|_| (dex_action(), amount(), amount()))
                .collect::<Vec<_>>()
        })
        .prop_map(|action_tuples| {
            let mut actions = Vec::new();
            let mut liquidity_provided = HashMap::<PairId, Coins>::new();

            for (action, amount1, amount2) in action_tuples {
                match &action {
                    DexAction::SwapExactAmountIn { route, .. }
                    | DexAction::SwapExactAmountOut { route, .. } => {
                        // Add liquidity for any pairs in the route that don't have it yet
                        for pair in route.inner().inner() {
                            if !liquidity_provided.contains_key(pair) {
                                let funds = coins! {
                                    pair.base_denom.clone() => amount1,
                                    pair.quote_denom.clone() => amount2,
                                };
                                let liquidity_action = DexAction::ProvideLiquidity {
                                    base_denom: pair.base_denom.clone(),
                                    quote_denom: pair.quote_denom.clone(),
                                    funds: funds.clone(),
                                };
                                actions.push(liquidity_action);
                                liquidity_provided.insert(pair.clone(), funds);
                            }
                        }
                        actions.push(action);
                    },
                    DexAction::ProvideLiquidity {
                        base_denom,
                        quote_denom,
                        funds,
                    } => {
                        let pair_id = PairId {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                        };
                        let current_funds = liquidity_provided.entry(pair_id).or_default();
                        current_funds
                            .insert_many(funds.clone().into_iter())
                            .unwrap();
                        actions.push(action);
                    },
                    _ => {
                        actions.push(action);
                    },
                }
            }

            actions
        })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        max_local_rejects: 1_000_000,
        max_global_rejects: 0,
        max_shrink_iters: 32,
        verbose: 1,
        ..ProptestConfig::default()
    })]

    #[test]
    fn dex_contract_tvl_equals_open_orders_plus_passive_liquidity(dex_actions in dex_actions(3, 5)) {
        let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption {
            bridge_ops: |accounts| {
                vec![
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(1_000_000_000_000_000),
                        recipient: accounts.user1.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::WETH_WARP,
                        },
                        amount: Uint128::new(1_000_000_000_000_000),
                        recipient: accounts.user1.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: solana::DOMAIN,
                            contract: solana::SOL_WARP,
                        },
                        amount: Uint128::new(1_000_000_000_000_000),
                        recipient: accounts.user1.address(),
                    },
                ]
            },
            ..Default::default()
        });

        // Query the balances of the user1 account.
        let balances = suite.query_balances(&accounts.user1)?;
        println!("balances: {:?}", balances);

        // Register fixed prices for all denoms.
        for denom in denoms() {
            register_fixed_price(
                &mut suite,
                &mut accounts,
                &contracts,
                denom,
                Udec128::ONE,
                6,
            );
        }

        // Create pairs
        suite.execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdatePairs(pair_ids().iter().map(|pair| PairUpdate {
                base_denom: pair.base_denom.clone(),
                quote_denom: pair.quote_denom.clone(),
                params: PairParams {
                    lp_denom: Denom::try_from(format!("dex/pool/{}/{}", pair.base_denom, pair.quote_denom)).unwrap(),
                    pool_type: PassiveLiquidity::Xyk {
                        order_spacing: Udec128::new_bps(1),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                },
            }).collect()),
            Coins::default(),
        )
        .should_succeed();


        // Check dex contract's balances. Should be empty.
        let balances = suite.query_balances(&contracts.dex)?;
        assert_eq!(balances, Coins::new());

        // Execute the actions.
        for action in dex_actions {
            action.execute(&mut suite, &mut accounts, &contracts);
        }

        // Create oracle querier
        let oracle_address = suite.query_oracle().unwrap();
        let mut oracle_querier = OracleQuerier::new_remote(oracle_address, suite.querier());

        // Check dex contract's TVL.
        let balances = suite.query_balances(&contracts.dex)?;
        println!("dex contract balances: {:?}", balances);
        let tvl = tvl_of_coins(&balances, &mut oracle_querier);

        // Query the open orders.
        let open_orders = suite.query_wasm_smart(contracts.dex, dex::QueryOrdersRequest {
            start_after: None,
            limit: None,
        })?;
        println!("open orders: {:?}", open_orders);
        let tvl_of_orders = tvl_of_orders(&open_orders, &mut oracle_querier);
        let mut order_balances = Coins::new();
        for (_, order) in open_orders {
            let denom = if order.direction == Direction::Bid {
                order.quote_denom
            } else {
                order.base_denom
            };
            order_balances.insert(Coin::new(denom, order.amount).unwrap()).unwrap();
        }
        println!("order balances: {:?}", order_balances);

        // Query the passive liquidity.
        let passive_liquidity = suite.query_wasm_smart(contracts.dex, dex::QueryReservesRequest {
            start_after: None,
            limit: None,
        })?;
        println!("passive liquidity: {:?}", passive_liquidity);
        let mut passive_liquidity_balances = Coins::new();
        for reserve in passive_liquidity.clone() {
            passive_liquidity_balances.insert_many(reserve.reserve).unwrap();
        }
        println!("passive liquidity balances: {:?}", passive_liquidity_balances);

        let tvl_of_passive_liquidity = tvl_of_reserves(&passive_liquidity, &mut oracle_querier);
        println!("tvl: {}", tvl);
        println!("tvl_of_orders: {}", tvl_of_orders);
        println!("tvl_of_passive_liquidity: {}", tvl_of_passive_liquidity);
        println!("tvl_of_orders + tvl_of_passive_liquidity: {}", tvl_of_orders + tvl_of_passive_liquidity);

        // Check that the TVL of the dex contract equals the TVL of the open orders plus the TVL of the passive liquidity.
        assert_approx_eq(tvl, tvl_of_orders + tvl_of_passive_liquidity, "0.01").unwrap();
    }
}
