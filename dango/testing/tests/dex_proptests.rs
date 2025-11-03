use {
    dango_genesis::{Contracts, DexOption, GenesisOption},
    dango_testing::{
        BridgeOp, Preset, TestAccounts, TestOption, TestSuite, setup_test_naive,
        setup_test_naive_with_custom_genesis,
    },
    dango_types::{
        constants::{dango, dango_usdc, eth, eth_usdc, sol, sol_usdc, usdc},
        dex::{
            self, CreateOrderRequest, Direction, Geometric, PairId, PairParams, PairUpdate,
            PassiveLiquidity, Price, SwapRoute, Xyk,
        },
        gateway::Remote,
    },
    grug::{
        Addressable, BlockOutcome, Bounded, Coin, Coins, Dec128_24, Denom, Inner, IsZero,
        MaxLength, Message, MultiplyFraction, NonEmpty, NonZero, Number, NumberConst, Permission,
        QuerierExt, ResultExt, Signed, Signer, Timestamp, Udec128, Udec128_6, Uint128, UniqueVec,
        ZeroInclusiveOneExclusive, btree_map, btree_set, coins,
    },
    grug_app::NaiveProposalPreparer,
    hyperlane_types::constants::{ethereum, solana},
    proptest::{prelude::*, proptest, sample::select},
    std::{
        collections::{BTreeMap, BTreeSet, HashMap, hash_map},
        fmt::Debug,
        ops::Sub,
        str::FromStr,
    },
};

/// Calculates the absolute difference between two values.
fn abs_diff<T>(a: T, b: T) -> <T as Sub>::Output
where
    T: Ord + Sub,
{
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
    if abs_diff(a, b) <= Uint128::new(5) {
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
    // println!("dex contract balances: {balances:?}");

    // Query the open orders.
    let open_orders = suite.query_wasm_smart(contracts.dex, dex::QueryOrdersRequest {
        start_after: None,
        limit: Some(u32::MAX),
    })?;
    // println!("open orders: {open_orders:?}");

    let mut order_balances = Coins::new();
    for (_, order) in open_orders {
        // Skip orders placed by the DEX contract itself, because those tokens
        // are already accounted for by the reserves.
        if order.user == contracts.dex {
            continue;
        }

        let (denom, amount) = match order.direction {
            Direction::Bid => {
                let remaining_in_quote = order.remaining.checked_mul_dec_ceil(order.price)?;
                (order.quote_denom, remaining_in_quote)
            },
            Direction::Ask => (order.base_denom, order.remaining),
        };

        order_balances.insert((denom, amount.into_int()))?;
    }
    // println!("order balances: {order_balances:?}");

    // Query the passive liquidity.
    let passive_liquidity = suite.query_wasm_smart(contracts.dex, dex::QueryReservesRequest {
        start_after: None,
        limit: None,
    })?;
    // println!("passive liquidity: {passive_liquidity:?}");

    let mut passive_liquidity_balances = Coins::new();
    for reserve in passive_liquidity.clone() {
        passive_liquidity_balances.insert_many(reserve.reserve)?;
    }
    // println!("passive liquidity balances: {passive_liquidity_balances:?}");

    // Check that the balances of the dex contract are equal to the balances of the open orders plus the balances of the passive liquidity.
    let mut order_and_passive_liquidity_balances = order_balances.clone();
    order_and_passive_liquidity_balances.insert_many(passive_liquidity_balances.clone())?;
    // println!("order_and_passive_liquidity_balances: {order_and_passive_liquidity_balances:?}");

    for coin in balances {
        let order_and_passive_liquidity_balance =
            order_and_passive_liquidity_balances.amount_of(&coin.denom);

        // println!("coin.denom: {}", coin.denom);
        // println!("coin.amount: {}", coin.amount);
        // println!("order_and_passive_liquidity_balance: {order_and_passive_liquidity_balance}");

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
        assert_approx_eq(coin.amount, order_and_passive_liquidity_balance, "0.001")?;
    }

    Ok(())
}

/// Checks that the open orders are equal to the recorded liquidity depths.
fn check_liquidity_depths(
    suite: &TestSuite<NaiveProposalPreparer>,
    contracts: &Contracts,
) -> Result<(), TestCaseError> {
    for pair_id in pair_ids() {
        // Query the pair params to get the bucket sizes and the base and quote denoms.
        let pair_params = suite.query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: pair_id.base_denom.clone(),
            quote_denom: pair_id.quote_denom.clone(),
        })?;
        let bucket_sizes = pair_params.bucket_sizes;
        let base_denom = pair_id.base_denom;
        let quote_denom = pair_id.quote_denom;

        // Query the open orders for this pair.
        let open_orders = suite.query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            start_after: None,
            limit: Some(u32::MAX),
        })?;

        // Sum the remaining order amounts by direction
        let mut bid_order_amount_base = Udec128_6::ZERO;
        let mut bid_order_amount_quote = Udec128_6::ZERO;
        let mut ask_order_amount_base = Udec128_6::ZERO;
        let mut ask_order_amount_quote = Udec128_6::ZERO;
        for (_, order) in open_orders {
            if order.direction == Direction::Bid {
                bid_order_amount_base += order.remaining;
                bid_order_amount_quote += order.remaining.checked_mul(order.price).unwrap();
            } else {
                ask_order_amount_base += order.remaining;
                ask_order_amount_quote += order.remaining.checked_mul(order.price).unwrap();
            }
        }

        // Query the liquidity depths for each bucket size and compare to the open orders.
        for bucket_size in bucket_sizes {
            let liquidity_depths =
                suite.query_wasm_smart(contracts.dex, dex::QueryLiquidityDepthRequest {
                    base_denom: base_denom.clone(),
                    quote_denom: quote_denom.clone(),
                    bucket_size: bucket_size.into_inner(),
                    limit: Some(u32::MAX),
                })?;

            let mut bid_depth_base = Udec128_6::ZERO;
            let mut bid_depth_quote = Udec128_6::ZERO;
            let mut ask_depth_base = Udec128_6::ZERO;
            let mut ask_depth_quote = Udec128_6::ZERO;

            if let Some(bid_depths) = liquidity_depths.bid_depth {
                for (_, depth) in bid_depths {
                    bid_depth_base.checked_add_assign(depth.depth_base).unwrap();
                    bid_depth_quote
                        .checked_add_assign(depth.depth_quote)
                        .unwrap();
                }
            }
            if let Some(ask_depths) = liquidity_depths.ask_depth {
                for (_, depth) in ask_depths {
                    ask_depth_base.checked_add_assign(depth.depth_base).unwrap();
                    ask_depth_quote
                        .checked_add_assign(depth.depth_quote)
                        .unwrap();
                }
            }

            // Assert that the liquidity depths are at most 1 unit apart from the open orders.
            assert!(bid_depth_base == bid_order_amount_base);
            assert!(bid_depth_quote == bid_order_amount_quote);
            assert!(ask_depth_base == ask_order_amount_base);
            assert!(ask_depth_quote == ask_order_amount_quote);
        }
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
        price: Price,
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
        input_denom: Denom,
    },
}

impl DexAction {
    fn execute(
        &self,
        suite: &mut TestSuite<NaiveProposalPreparer>,
        accounts: &mut TestAccounts,
        contracts: &Contracts,
    ) -> Result<Option<BlockOutcome>, TestCaseError> {
        println!("Executing action: {self:?}");

        let block_outcome = match self {
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
                        creates: vec![CreateOrderRequest::new_limit(
                            base_denom.clone(),
                            quote_denom.clone(),
                            *direction,
                            NonZero::new(*price)?,
                            NonZero::new(deposit.amount)?,
                        )],
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

                if let Err(err) = &block_outcome.tx_outcomes.first().unwrap().result {
                    println!("CreateLimitOrder error: {err}");
                    println!("block_outcome: {block_outcome:?}");
                }

                assert!(
                    block_outcome
                        .tx_outcomes
                        .iter()
                        .all(|tx_outcome| tx_outcome.result.is_ok())
                );

                Some(block_outcome)
            },
            DexAction::CreateMarketOrder {
                base_denom,
                quote_denom,
                direction,
                amount,
            } => {
                let max_slippage = Bounded::<Udec128, ZeroInclusiveOneExclusive>::new(
                    Udec128::from_str("0.999999").unwrap(),
                )
                .unwrap();

                // Query resting order book
                let resting_order_book = suite
                    .query_wasm_smart(contracts.dex, dex::QueryRestingOrderBookStateRequest {
                        base_denom: base_denom.clone(),
                        quote_denom: quote_denom.clone(),
                    })
                    .unwrap();
                // println!("resting order book: {resting_order_book:?}");

                let deposit = match direction {
                    Direction::Bid => {
                        if resting_order_book.best_ask_price.is_none() {
                            return Ok(None);
                        }
                        let best_ask_price = resting_order_book.best_ask_price.unwrap();

                        let one_add_max_slippage = Price::ONE.saturating_add(*max_slippage);
                        let price = best_ask_price.saturating_mul(one_add_max_slippage);

                        Coin {
                            denom: quote_denom.clone(),
                            amount: amount.checked_mul_dec_ceil(price)?,
                        }
                    },
                    Direction::Ask => {
                        if resting_order_book.best_bid_price.is_none() {
                            return Ok(None);
                        }

                        Coin {
                            denom: base_denom.clone(),
                            amount: *amount,
                        }
                    },
                };

                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates: vec![CreateOrderRequest::new_market(
                            base_denom.clone(),
                            quote_denom.clone(),
                            *direction,
                            max_slippage,
                            NonZero::new(deposit.amount).unwrap(),
                        )],
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

                assert!(
                    block_outcome
                        .tx_outcomes
                        .iter()
                        .all(|tx_outcome| tx_outcome.result.is_ok())
                );

                Some(block_outcome)
            },
            DexAction::ProvideLiquidity {
                base_denom,
                quote_denom,
                funds,
            } => {
                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::ProvideLiquidity {
                        base_denom: base_denom.clone(),
                        quote_denom: quote_denom.clone(),
                        minimum_output: None,
                    },
                    funds.clone(),
                )
                .unwrap();

                let tx = accounts
                    .user1
                    .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
                    .unwrap();

                let block_outcome = suite.make_block(vec![tx]).block_outcome;

                if let Err(err) = &block_outcome.tx_outcomes.first().unwrap().result {
                    println!("ProvideLiquidity error: {err}");
                    println!("block_outcome: {block_outcome:?}");
                }

                assert!(
                    block_outcome
                        .tx_outcomes
                        .iter()
                        .all(|tx_outcome| tx_outcome.result.is_ok())
                );

                Some(block_outcome)
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

                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::WithdrawLiquidity {
                        base_denom: base_denom.clone(),
                        quote_denom: quote_denom.clone(),
                        minimum_output: None,
                    },
                    Coins::one(pair.lp_denom.clone(), lp_token_amount).unwrap(),
                )
                .unwrap();

                let tx = accounts
                    .user1
                    .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
                    .unwrap();

                let block_outcome = suite.make_block(vec![tx]).block_outcome;

                assert!(
                    block_outcome
                        .tx_outcomes
                        .iter()
                        .all(|tx_outcome| tx_outcome.result.is_ok())
                );

                Some(block_outcome)
            },
            DexAction::SwapExactAmountIn { route, input } => {
                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::SwapExactAmountIn {
                        route: route.clone(),
                        minimum_output: None,
                    },
                    input.clone(),
                )
                .unwrap();

                let tx = accounts
                    .user1
                    .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
                    .unwrap();

                let block_outcome = suite.make_block(vec![tx]).block_outcome;

                block_outcome
                    .tx_outcomes
                    .first()
                    .unwrap()
                    .clone()
                    .should(|tx_outcome| {
                        // We expect the transaction to succeed, unless for the
                        // following four specific reasons. These errors indicate
                        // an unfortunate combination of parameters, not an bug
                        // in the contract.
                        if let Err(err) = &tx_outcome.result {
                            [
                                "insufficient liquidity",
                                "output amount after fee must be positive", // this refers to the output after _liquidity fee_
                                "output amount is zero",                    // this refers to the output after _protocol fee_
                                "not enough liquidity to fulfill the swap!"
                            ]
                            .iter()
                            .any(|reason| err.error.contains(reason))
                        } else {
                            true
                        }
                    });
                Some(block_outcome)
            },
            DexAction::SwapExactAmountOut {
                route,
                output,
                input_denom,
            } => {
                // Query the input denom balance of the user1 account.
                let balance = suite
                    .query_balance(&accounts.user1.address(), input_denom.clone())
                    .unwrap();

                // Set funds amount to the balance
                let funds = Coins::one(input_denom.clone(), balance).unwrap();

                let msg = Message::execute(
                    contracts.dex,
                    &dex::ExecuteMsg::SwapExactAmountOut {
                        route: route.clone(),
                        output: NonZero::new(output.clone()).unwrap(),
                    },
                    funds,
                )
                .unwrap();

                let tx = accounts
                    .user1
                    .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
                    .unwrap();

                let block_outcome = suite.make_block(vec![tx]).block_outcome;

                block_outcome
                    .tx_outcomes
                    .first()
                    .unwrap()
                    .clone()
                    .should(|tx_outcome| {
                        // We expect the transaction to succeed, unless for the
                        // the following three specific reasons:
                        if let Err(err) = &tx_outcome.result {
                            [
                                "insufficient liquidity",
                                "input amount must be positive",
                                "not enough liquidity to fulfill the swap",
                            ]
                            .iter()
                            .any(|reason| err.error.contains(reason))
                        } else {
                            true
                        }
                    });

                Some(block_outcome)
            },
        };

        Ok(block_outcome)
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

/// Bucket sizes for each pair
fn bucket_sizes() -> BTreeMap<PairId, BTreeSet<NonZero<Price>>> {
    btree_map! {
        PairId {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        } => btree_set! {
            dango_usdc::ONE_THOUSANDTH,
            dango_usdc::ONE_HUNDREDTH,
            dango_usdc::ONE_TENTH,
            dango_usdc::ONE,
            dango_usdc::TEN,
            dango_usdc::FIFTY,
            dango_usdc::ONE_HUNDRED,
        },
        PairId {
            base_denom: sol::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        } => btree_set! {
            sol_usdc::ONE_HUNDREDTH,
            sol_usdc::ONE_TENTH,
            sol_usdc::ONE,
            sol_usdc::TEN,
        },
        PairId {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        } => btree_set! {
            eth_usdc::ONE_HUNDREDTH,
            eth_usdc::ONE_TENTH,
            eth_usdc::ONE,
            eth_usdc::TEN,
            eth_usdc::FIFTY,
            eth_usdc::ONE_HUNDRED,
        },
    }
}

/// Proptest strategy for generating a pair id
fn pair_id() -> impl Strategy<Value = PairId> {
    select(pair_ids())
}

/// Proptest strategy for generating a pool type
fn pool_type() -> impl Strategy<Value = PassiveLiquidity> {
    prop_oneof![
        Just(PassiveLiquidity::Xyk(Xyk {
            spacing: Udec128::new_bps(1000),
            reserve_ratio: Bounded::new_unchecked(Udec128::new_percent(1)),
            limit: 30,
        })),
        Just(PassiveLiquidity::Geometric(Geometric {
            ratio: Bounded::new(Udec128::new_percent(50)).unwrap(),
            spacing: Udec128::new_percent(50),
            limit: 10,
        })),
    ]
}

/// Proptest strategy for generating a vec of pool types
fn pool_types(length: usize) -> impl Strategy<Value = Vec<PassiveLiquidity>> {
    (0..=length)
        .collect::<Vec<_>>()
        .into_iter()
        .map(|_| pool_type())
        .collect::<Vec<_>>()
}

/// Proptest strategy for generating an order direction
fn direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Bid), Just(Direction::Ask)]
}

/// Proptest strategy for generating an amount between 1 and 1 billion microunits
fn amount() -> impl Strategy<Value = Uint128> {
    (1u128..1_000_000_000u128).prop_map(Uint128::new)
}

/// Proptest strategy for generating a price as [-3, 3] permille from 1.0
fn price() -> impl Strategy<Value = Price> {
    (-3i128..3i128).prop_map(|price_diff| {
        (Dec128_24::ONE - Dec128_24::new_permille(price_diff))
            .checked_into_unsigned()
            .unwrap()
    })
}

// /// Proptest strategy for generating an arbitrary price between min and max possible prices.
// fn price() -> impl Strategy<Value = Price> {
//     (1u128..u128::MAX).prop_map(|raw_price| Price::raw(Uint128::new(raw_price)))
// }

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

fn limit_order() -> impl Strategy<Value = DexAction> {
    (price(), pair_id(), direction(), amount()).prop_map(
        move |(price, pair_id, direction, amount)| DexAction::CreateLimitOrder {
            base_denom: pair_id.base_denom,
            quote_denom: pair_id.quote_denom,
            direction,
            amount,
            price,
        },
    )
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
        limit_order(),
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
                    input_denom,
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

/// Proptest strategy for generating a list of LimitOrder actions
#[allow(dead_code)]
fn limit_orders(min_size: usize, max_size: usize) -> impl Strategy<Value = Vec<DexAction>> {
    (min_size..=max_size).prop_flat_map(move |size| {
        (1..=size)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|_| limit_order())
            .collect::<Vec<_>>()
    })
}

/// Feed fixed oracle prices for all denoms.
fn feed_prices(
    timestamp: Timestamp,
    suite: &mut TestSuite<NaiveProposalPreparer>,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> Result<(), TestCaseError> {
    for denom in denoms() {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &dango_types::oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                    denom => dango_types::oracle::PriceSource::Fixed {
                        humanized_price: Udec128::ONE,
                        precision: 6,
                        timestamp,
                    },
                }),
                Coins::default(),
            )
            .should_succeed();
    }

    Ok(())
}

/// Test a list of DexActions. Execute the actions and check balances after each action.
fn test_dex_actions(
    dex_actions: Vec<DexAction>,
    pool_types: Vec<PassiveLiquidity>,
) -> Result<(TestSuite<NaiveProposalPreparer>, TestAccounts, Contracts), TestCaseError> {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive_with_custom_genesis(
        TestOption {
            bridge_ops: |accounts| {
                vec![
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(u128::MAX),
                        recipient: accounts.user1.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::WETH_WARP,
                        },
                        amount: Uint128::new(u128::MAX),
                        recipient: accounts.user1.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: solana::DOMAIN,
                            contract: solana::SOL_WARP,
                        },
                        amount: Uint128::new(u128::MAX),
                        recipient: accounts.user1.address(),
                    },
                ]
            },
            ..Default::default()
        },
        GenesisOption {
            dex: DexOption {
                permissions: dango_types::dex::Permissions {
                    swap: dango_types::dex::PairPermissions {
                        permissions: vec![],
                        default_permission: Permission::Everybody,
                    },
                },
                ..Preset::preset_test()
            },
            ..Preset::preset_test()
        },
    );

    // Print user address
    println!("user1 address: {}", accounts.user1.address());

    // Print dex contract address
    println!("dex contract address: {}", contracts.dex);

    // Query the balances of the user1 account.
    let balances = suite.query_balances(&accounts.user1)?;
    println!("user1 balances: {balances:?}");

    // Register fixed prices for all denoms.
    let timestamp = Timestamp::from_nanos(u128::MAX); // Maximum time in the future to prevent oracle price from being outdated.
    feed_prices(timestamp, &mut suite, &mut accounts, &contracts)?;

    let bucket_sizes = bucket_sizes();

    // Create pairs
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(
                pair_ids()
                    .iter()
                    .zip(pool_types.iter())
                    .map(|(pair, pool_type)| PairUpdate {
                        base_denom: pair.base_denom.clone(),
                        quote_denom: pair.quote_denom.clone(),
                        params: PairParams {
                            lp_denom: Denom::try_from(format!(
                                "dex/pool/{}/{}",
                                pair.base_denom, pair.quote_denom
                            ))
                            .unwrap(),
                            pool_type: pool_type.clone(),
                            bucket_sizes: bucket_sizes.get(pair).unwrap().clone(),
                            swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                            min_order_size: Uint128::ZERO,
                        },
                    })
                    .collect(),
            )),
            Coins::default(),
        )
        .should_succeed();

    // Check dex contract's balances. Should be empty.
    let balances = suite.query_balances(&contracts.dex)?;
    assert!(balances.is_empty());

    // Execute the actions and check balances after each action.
    for action in dex_actions {
        // Execute the action.
        let block_outcome = action.execute(&mut suite, &mut accounts, &contracts)?;

        // Query dex paused status.
        let is_paused = suite
            .query_wasm_smart(contracts.dex, dex::QueryPausedRequest {})
            .should_succeed();

        // Print block outcome if cron outcomes failed or if dex is paused
        if let Some(block_outcome) = &block_outcome {
            if block_outcome
                .cron_outcomes
                .iter()
                .any(|cron_outcome| cron_outcome.cron_event.as_result().is_err())
                || is_paused
            {
                println!("Failed cron outcome or dex is paused. Block outcome: {block_outcome:?}");
            }
        }

        // Ensure all cron outcomes succeeded
        if let Some(block_outcome) = &block_outcome {
            assert!(
                block_outcome
                    .cron_outcomes
                    .iter()
                    .all(|cron_outcome| cron_outcome.cron_event.as_result().is_ok())
            );
        }

        // Ensure dex is not paused after executing the last action.
        assert!(!is_paused);

        // Check balances.
        check_balances(&suite, &contracts)?;

        // Check liquidity depths.
        check_liquidity_depths(&suite, &contracts)?;
    }

    Ok((suite, accounts, contracts))
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_local_rejects: 1_000_000,
        max_global_rejects: 0,
        max_shrink_iters: 32,
        verbose: 1,
        ..ProptestConfig::default()
    })]

    #[test]
    fn dex_contract_balances_equals_open_orders_plus_passive_liquidity(dex_actions in dex_actions(5, 10), pool_types in pool_types(3)) {
        test_dex_actions(dex_actions, pool_types)?;
    }

    #[test]
    fn provide_liq_and_market_order(dex_actions in provide_liquidity_and_market_order(), pool_types in pool_types(3)) {
        test_dex_actions(dex_actions, pool_types)?;
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
    let (suite, _, contracts) = test_dex_actions(
        vec![
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
        ],
        vec![PassiveLiquidity::Xyk(Xyk {
            spacing: Udec128::new_bps(1000),
            reserve_ratio: Bounded::new_unchecked(Udec128::new_percent(1)),
            limit: 30,
        })],
    )
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
