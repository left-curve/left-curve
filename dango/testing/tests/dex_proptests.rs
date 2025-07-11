use {
    dango_genesis::Contracts,
    dango_testing::{
        BridgeOp, TestAccounts, TestOption, TestSuite, constants::MOCK_GENESIS_TIMESTAMP,
        setup_test_naive,
    },
    dango_types::{
        constants::{dango, eth, sol, usdc},
        dex::{
            self, CreateLimitOrderRequest, CreateMarketOrderRequest, Direction, PairId, PairParams,
            PairUpdate, PassiveLiquidity, SwapRoute,
        },
        gateway::Remote,
    },
    grug::{
        Addressable, Bounded, Coin, Coins, Dec128, Denom, Inner, IsZero, MaxLength, Message,
        MultiplyFraction, NextNumber, NonEmpty, NonZero, NumberConst, PrevNumber, QuerierExt,
        ResultExt, Signed, Signer, Udec128, Udec256, Uint128, UniqueVec, btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    hyperlane_types::constants::{ethereum, solana},
    proptest::{prelude::*, proptest, sample::select},
    std::{
        collections::{HashMap, hash_map},
        fmt::Debug,
        str::FromStr,
    },
};

/// Calculates the absolute difference between two values.
fn absolute_difference(a: Uint128, b: Uint128) -> Uint128 {
    if a > b {
        a - b
    } else {
        b - a
    }
}

/// Calculates the relative difference between two values.
fn relative_difference(a: Uint128, b: Uint128) -> Udec128 {
    // Handle the case where both numbers are zero
    if a == Uint128::ZERO && b == Uint128::ZERO {
        return Udec128::ZERO;
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
    Udec128::checked_from_ratio(abs_diff, larger).unwrap()
}

/// Asserts that two values are approximately equal within a specified
/// relative difference.
fn assert_approx_eq(a: Uint128, b: Uint128, max_rel_diff: &str) -> Result<(), TestCaseError> {
    // An absolute difference of up to a few units is acceptable, and unavoidable
    // due to rounding errors. In this case, we consider the values effectively equal.
    if absolute_difference(a, b) <= Uint128::new(5) {
        return Ok(());
    }

    // If the difference is greater than one unit, we ensure the relative difference
    // isn't greater than a given threshold.
    let rel_diff_num = relative_difference(a, b);
    let rel_diff = Udec128::from_str(rel_diff_num.to_string().as_str()).unwrap();
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

/// Checks that the balances of the dex contract are equal to the balances of the open orders plus the balances of the passive liquidity.
fn check_balances(
    suite: &TestSuite<NaiveProposalPreparer>,
    contracts: &Contracts,
) -> Result<(), TestCaseError> {
    // Check dex contract's balances.
    let balances = suite.query_balances(&contracts.dex)?;
    println!("dex contract balances: {balances:?}");

    // Query the open orders.
    let open_orders = suite.query_wasm_smart(contracts.dex, dex::QueryOrdersRequest {
        start_after: None,
        limit: None,
    })?;
    println!("open orders: {open_orders:?}");

    let mut order_balances = Coins::new();
    for (_, order) in open_orders {
        let (denom, amount) = match order.direction {
            Direction::Bid => {
                let remaining_in_quote = order.remaining.checked_mul_dec_ceil(order.price)?;
                (order.quote_denom, remaining_in_quote)
            },
            Direction::Ask => (order.base_denom, order.remaining),
        };

        order_balances.insert((denom, amount.into_int().checked_into_prev().unwrap()))?;
    }
    println!("order balances: {order_balances:?}");

    // Query the passive liquidity.
    let passive_liquidity = suite.query_wasm_smart(contracts.dex, dex::QueryReservesRequest {
        start_after: None,
        limit: None,
    })?;
    println!("passive liquidity: {passive_liquidity:?}");

    let mut passive_liquidity_balances = Coins::new();
    for reserve in passive_liquidity.clone() {
        passive_liquidity_balances.insert_many(reserve.reserve)?;
    }
    println!("passive liquidity balances: {passive_liquidity_balances:?}");

    // Check that the balances of the dex contract are equal to the balances of the open orders plus the balances of the passive liquidity.
    let mut order_and_passive_liquidity_balances = order_balances.clone();
    order_and_passive_liquidity_balances.insert_many(passive_liquidity_balances.clone())?;
    println!("order_and_passive_liquidity_balances: {order_and_passive_liquidity_balances:?}");

    for coin in balances {
        let order_and_passive_liquidity_balance =
            order_and_passive_liquidity_balances.amount_of(&coin.denom);

        println!("coin.denom: {}", coin.denom);
        println!("coin.amount: {}", coin.amount);
        println!("order_and_passive_liquidity_balance: {order_and_passive_liquidity_balance}");

        // Ensure contract is not undercollateralized.
        assert!(coin.amount >= order_and_passive_liquidity_balance);

        // We don't care about LP tokens.
        if coin
            .denom
            .starts_with(&[dex::NAMESPACE.clone(), dex::LP_NAMESPACE.clone()])
        {
            continue;
        }

        // Dex contract sometimes has some dust amounts, so we ignore them.
        if coin.amount < Uint128::new(10) && order_and_passive_liquidity_balance == Uint128::ZERO {
            continue;
        }

        // Assert that the balance of the dex contract equals the balance of the open orders plus the balance of the passive liquidity.
        assert_approx_eq(coin.amount, order_and_passive_liquidity_balance, "0.0001")?;
    }

    Ok(())
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
    CreateMarketOrder {
        base_denom: Denom,
        quote_denom: Denom,
        direction: Direction,
        amount: Uint128,
    },
    ProvideLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
        funds: Coins,
    },
    WithdrawLiquidity {
        base_denom: Denom,
        quote_denom: Denom,
        fraction_of_lp_tokens: Udec128,
    },
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
    ) -> Result<(), TestCaseError> {
        println!("Executing action: {self:?}");

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
                        amount: amount.checked_mul_dec_ceil(*price)?,
                    },
                    Direction::Ask => Coin {
                        denom: base_denom.clone(),
                        amount: *amount,
                    },
                };

                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates_market: vec![],
                        creates_limit: vec![CreateLimitOrderRequest {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                            direction: *direction,
                            amount: NonZero::new(*amount)?,
                            price: price.into_next(),
                        }],
                        cancels: None,
                    },
                    Coins::one(deposit.denom, deposit.amount)?,
                )
                .unwrap();

                let tx = accounts
                    .user1
                    .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
                    .unwrap();

                let block_outcome = suite.make_block(vec![tx]).block_outcome;
                // println!("block outcome: {block_outcome:?}");

                assert!(
                    block_outcome
                        .cron_outcomes
                        .first()
                        .unwrap()
                        .cron_event
                        .as_result()
                        .is_ok()
                );
            },
            DexAction::CreateMarketOrder {
                base_denom,
                quote_denom,
                direction,
                amount,
            } => {
                let deposit = match direction {
                    Direction::Bid => Coin {
                        denom: quote_denom.clone(),
                        amount: *amount,
                    },
                    Direction::Ask => Coin {
                        denom: base_denom.clone(),
                        amount: *amount,
                    },
                };

                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates_market: vec![CreateMarketOrderRequest {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                            direction: *direction,
                            amount: NonZero::new(*amount).unwrap(),
                            max_slippage: Udec128::MAX,
                        }],
                        creates_limit: vec![],
                        cancels: None,
                    },
                    Coins::one(deposit.denom, deposit.amount).unwrap(),
                )
                .unwrap();

                let tx = accounts
                    .user1
                    .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
                    .unwrap();

                let block_outcome = suite.make_block(vec![tx]).block_outcome;
                // println!("block outcome: {block_outcome:?}");

                assert!(
                    block_outcome
                        .cron_outcomes
                        .first()
                        .unwrap()
                        .cron_event
                        .as_result()
                        .is_ok()
                );
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
            DexAction::WithdrawLiquidity {
                base_denom,
                quote_denom,
                fraction_of_lp_tokens,
            } => {
                // Query pair to get LP token denom
                let pair = suite
                    .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
                        base_denom: base_denom.clone(),
                        quote_denom: quote_denom.clone(),
                    })
                    .unwrap();

                let lp_token_balance = suite
                    .query_balance(&accounts.user1.address(), pair.lp_denom.clone())
                    .unwrap();
                let lp_token_amount = lp_token_balance
                    .checked_mul_dec_ceil(*fraction_of_lp_tokens)
                    .unwrap();

                suite
                    .execute(
                        &mut accounts.user1,
                        contracts.dex,
                        &dex::ExecuteMsg::WithdrawLiquidity {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                        },
                        Coins::one(pair.lp_denom.clone(), lp_token_amount).unwrap(),
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
                        // We expect the transaction to succeed, unless for the
                        // following three specific reasons. These errors indicate
                        // an unfortunate combination of parameters, not an bug
                        // in the contract.
                        if let Err(err) = &tx_outcome.result {
                            err.contains("insufficient liquidity")
                                || err.contains("output amount after fee must be positive") // this refers to output after _liquidity fee_
                                || err.contains("output amount is zero") // this refers to output after _protocol fee_
                        } else {
                            true
                        }
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
                        // We expect the transaction to succeed, unless for two
                        // specific reasons:
                        if let Err(err) = &tx_outcome.result {
                            err.contains("insufficient liquidity")
                                || err.contains("input amount must be positive")
                        } else {
                            true
                        }
                    });
            },
        }

        Ok(())
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

/// Proptest strategy for generating an amount between 10000 and 1 billion microunits
fn amount() -> impl Strategy<Value = Uint128> {
    (10_000u128..1_000_000_000u128).prop_map(Uint128::new)
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

/// Proptest strategy for generating a ProvideLiquidity action
fn provide_liquidity() -> impl Strategy<Value = DexAction> {
    (pair_id(), amount(), amount()).prop_map(move |(pair_id, amount1, amount2)| {
        DexAction::ProvideLiquidity {
            base_denom: pair_id.base_denom.clone(),
            quote_denom: pair_id.quote_denom.clone(),
            funds: coins! {
                pair_id.base_denom => amount1,
                pair_id.quote_denom => amount2,
            },
        }
    })
}

/// Proptest strategy for generating a MarketOrder action with a specific pair id
fn market_order_with_pair_id(pair_id: PairId) -> impl Strategy<Value = DexAction> {
    (direction(), amount()).prop_map(move |(direction, amount)| DexAction::CreateMarketOrder {
        base_denom: pair_id.base_denom.clone(),
        quote_denom: pair_id.quote_denom.clone(),
        direction,
        amount,
    })
}

/// Proptest strategy for generating a MarketOrder action
fn market_order() -> impl Strategy<Value = DexAction> {
    (pair_id()).prop_flat_map(market_order_with_pair_id)
}

/// Proptest strategy for generating a ProvideLiquidity and MarketOrder action
/// where the MarketOrder is created for the same pair as the ProvideLiquidity.
fn provide_liquidity_and_market_order() -> impl Strategy<Value = Vec<DexAction>> {
    provide_liquidity().prop_flat_map(|provide_liquidity| {
        let pair_id = match provide_liquidity {
            DexAction::ProvideLiquidity {
                ref base_denom,
                ref quote_denom,
                funds: _,
            } => PairId {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
            },
            _ => panic!("`provide_liquidity` should be a `ProvideLiquidity` action"),
        };
        market_order_with_pair_id(pair_id)
            .prop_map(move |market_order| vec![provide_liquidity.clone(), market_order])
    })
}

/// Proptest strategy for generating a DexAction
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
        market_order(),
        provide_liquidity(),
        (pair_id(), 1u128..95u128).prop_map(move |(pair_id, fraction)| {
            DexAction::WithdrawLiquidity {
                base_denom: pair_id.base_denom.clone(),
                quote_denom: pair_id.quote_denom.clone(),
                fraction_of_lp_tokens: Udec128::new_percent(fraction),
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

/// Proptest strategy for generating a list of DexActions
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
                    DexAction::WithdrawLiquidity {
                        base_denom,
                        quote_denom,
                        fraction_of_lp_tokens: _,
                    } => {
                        let pair = PairId {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                        };
                        if let hash_map::Entry::Vacant(e) = liquidity_provided.entry(pair.clone()) {
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
                            e.insert(funds);
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

/// Test a list of DexActions. Execute the actions and check balances after each action.
fn test_dex_actions(
    dex_actions: Vec<DexAction>,
) -> Result<(TestSuite<NaiveProposalPreparer>, TestAccounts, Contracts), TestCaseError> {
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

    // Print user address
    println!("user1 address: {}", accounts.user1.address());

    // Print dex contract address
    println!("dex contract address: {}", contracts.dex);

    // Query the balances of the user1 account.
    let balances = suite.query_balances(&accounts.user1)?;
    println!("user1 balances: {balances:?}");

    // Register fixed prices for all denoms.
    for denom in denoms() {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                    denom => dango_types::oracle::PriceSource::Fixed {
                        humanized_price: Udec256::ONE,
                        precision: 6,
                        // Use a very recent time to avoid the "price is too old" error.
                        timestamp: MOCK_GENESIS_TIMESTAMP,
                    },
                }),
                Coins::default(),
            )
            .should_succeed();
    }

    // Create pairs
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdatePairs(
                pair_ids()
                    .iter()
                    .map(|pair| PairUpdate {
                        base_denom: pair.base_denom.clone(),
                        quote_denom: pair.quote_denom.clone(),
                        params: PairParams {
                            lp_denom: Denom::try_from(format!(
                                "dex/pool/{}/{}",
                                pair.base_denom, pair.quote_denom
                            ))
                            .unwrap(),
                            pool_type: PassiveLiquidity::Xyk {
                                order_spacing: Udec128::new_bps(1000),
                                reserve_ratio: Bounded::new_unchecked(Udec128::new_percent(1)),
                            },
                            swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                        },
                    })
                    .collect(),
            ),
            Coins::default(),
        )
        .should_succeed();

    // Check dex contract's balances. Should be empty.
    let balances = suite.query_balances(&contracts.dex)?;
    assert!(balances.is_empty());

    // Execute the actions and check balances after each action.
    for action in dex_actions {
        // Execute the action.
        action.execute(&mut suite, &mut accounts, &contracts)?;

        // Check balances.
        check_balances(&suite, &contracts)?;
    }

    Ok((suite, accounts, contracts))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10_000,
        max_local_rejects: 1_000_000,
        max_global_rejects: 0,
        max_shrink_iters: 0,
        verbose: 1,
        ..ProptestConfig::default()
    })]

    #[ignore = "this test takes 15+ minutes so skip it during CI"]
    #[test]
    fn dex_contract_balances_equals_open_orders_plus_passive_liquidity(dex_actions in dex_actions(5, 10)) {
        test_dex_actions(dex_actions)?;
    }

    #[ignore = "this test takes 15+ minutes so skip it during CI"]
    #[test]
    fn provide_liq_and_market_order(dex_actions in provide_liquidity_and_market_order()) {
        test_dex_actions(dex_actions)?;
    }
}

/// An error case discovered by proptest. Here the traders makes a very large
/// market order, dumping as much as ~6x the liqudity available in the pool.
/// Before the fix, this would reduce the pool's USDC liquidity to zero, causing
/// any subsequent liquidity provision to fail with "division by zero" error.
///
/// We've introduced a fix this way: introduce a new parameter `reserve_ratio`
/// to the xyk pool, which identifies the portion of funds that the pool must
/// hold and use to place order. E.g. if reserve ratio is 5%, then the pool will
/// only use 95% of its funds to place order, thus its liquidity never reduces
/// to zero.
#[test]
fn xyk_liquidity_should_not_reduce_to_zero_by_market_order() {
    let (suite, _, contracts) = test_dex_actions(vec![
        DexAction::ProvideLiquidity {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            funds: coins! {
                eth::DENOM.clone() => Uint128::new(106265421),
                usdc::DENOM.clone() => Uint128::new(58295192),
            },
        },
        DexAction::CreateMarketOrder {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(626970560),
        },
    ])
    .unwrap();

    // Query the reserve of ETH-USDC pool. Neither tokens should be zero.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryReserveRequest {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|reserve| {
            reserve.amount_of(&eth::DENOM).unwrap().is_non_zero()
                && reserve.amount_of(&usdc::DENOM).unwrap().is_non_zero()
        });
}
