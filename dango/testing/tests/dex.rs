use {
    dango_oracle::{PRICE_SOURCES, PRICES},
    dango_testing::{BridgeOp, TestOption, setup_test_naive},
    dango_types::{
        account::single::Params,
        account_factory::AccountParams,
        config::AppConfig,
        constants::{atom, dango, eth, usdc, xrp},
        dex::{
            self, CancelOrderRequest, CreateLimitOrderRequest, CreateMarketOrderRequest, Direction,
            OrderId, OrderResponse, PairId, PairParams, PairUpdate, PassiveLiquidity,
            QueryOrdersByPairRequest, QueryOrdersRequest, QueryReserveRequest,
            QueryRestingOrderBookStateRequest, RestingOrderBookState,
        },
        gateway::Remote,
        oracle::{self, PrecisionlessPrice, PriceSource},
    },
    grug::{
        Addr, Addressable, BalanceChange, Bounded, Coin, CoinPair, Coins, Denom, Fraction, Inner,
        MaxLength, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, QuerierExt,
        ResultExt, Signer, StdError, StdResult, Timestamp, Udec128, Udec128_6, Udec128_24, Uint128,
        UniqueVec, btree_map, coin_pair, coins,
    },
    hyperlane_types::constants::ethereum,
    pyth_types::constants::USDC_USD_ID,
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
    },
    test_case::test_case,
};

/// Ensure order amounts can't be zero.
///
/// Typically this should fail during `CheckTx`, because the tx would fail to
/// deserialize. However, let's just assume an attacker somehow circumvents that
/// (which would require him to collude with a validator), then the contract
/// would still reject the order.
#[test]
fn cannot_submit_order_with_zero_amount() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Attempt to submit a limit order with zero amount.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::ZERO), // incorrect!
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 1).unwrap(),
        )
        .should_fail_with_error(StdError::zero_value::<Uint128>());

    // Attempt to submit a market order with zero amount.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::ZERO), // incorrect!
                    max_slippage: Udec128::ZERO,
                }],
                creates_limit: vec![],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 1).unwrap(),
        )
        .should_fail_with_error(StdError::zero_value::<Uint128>());
}

#[test]
fn cannot_submit_orders_in_non_existing_pairs() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: atom::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 1).unwrap(),
        )
        .should_fail_with_error(format!(
            "pair not found with base `{}` and quote `{}`",
            atom::DENOM.clone(),
            usdc::DENOM.clone()
        ));
}

// Test cases from:
// https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better
//
// --------------------------------- example 1 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 10), // order_id = !0
        (Direction::Bid, 20, 10), // !1
        (Direction::Bid, 10, 10), // !2
        (Direction::Ask, 10, 10), // 3
        (Direction::Ask, 20, 10), // 4
        (Direction::Ask, 30, 10), // 5
    ],
    btree_map! {
        OrderId::new(!3) => 10,
        OrderId::new(6)  => 10,
    },
    btree_map! {
        OrderId::new(!1) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        OrderId::new(!2) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        OrderId::new(!3) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        OrderId::new(4) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(199), // Receives one less due to fee
        },
        OrderId::new(5) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(199), // Receives one less due to fee
        },
        OrderId::new(6) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Unchanged,
        },
    };
    "example 1"
)]
// --------------------------------- example 2 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 10), // !0
        (Direction::Bid, 20, 10), // !1
        (Direction::Bid, 10, 10), // !2
        (Direction::Ask,  5, 10), //  3
        (Direction::Ask, 15, 10), //  4
        (Direction::Ask, 25, 10), //  5
    ],
    btree_map! {
        OrderId::new(!3) => 10,
        OrderId::new(6)  => 10,
    },
    btree_map! {
        OrderId::new(!1) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(175),
        },
        OrderId::new(!2) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(175),
        },
        OrderId::new(!3) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        OrderId::new(4) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        OrderId::new(5) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        OrderId::new(6) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Unchanged,
        },
    };
    "example 2"
)]
// --------------------------------- example 3 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 10), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - 50% filled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask,  5, 10), //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - unfilled
        (Direction::Bid, 30,  5), // !6 - filled
    ],
    btree_map! {
        OrderId::new(!2) =>  5,
        OrderId::new(!3) => 10,
        OrderId::new(6)  => 10,
    },
    btree_map! {
        OrderId::new(!1) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(175),
        },
        OrderId::new(!2) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(4),   // half filled, receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(188), // -200 deposit, +12 refund
        },
        OrderId::new(!3) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        OrderId::new(4) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        OrderId::new(5) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174),
        },
        OrderId::new(6) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Unchanged,
        },
        OrderId::new(!7) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(4),
            usdc::DENOM.clone()  => BalanceChange::Decreased(88), // -150 deposit, +62 refund
        },
    };
    "example 3"
)]
// --------------------------------- example 4 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 20), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask,  5, 10), //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - unfilled
    ],
    btree_map! {
        OrderId::new(!2) => 10,
        OrderId::new(!3) => 10,
        OrderId::new(6)  => 10,
    },
    btree_map! {
        OrderId::new(!1) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(19), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(450), // -600 deposit, +150 refund
        },
        OrderId::new(!2) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        OrderId::new(!3) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        OrderId::new(4) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(224), // Receives one less due to fee
        },
        OrderId::new(5) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(224),
        },
        OrderId::new(6) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Unchanged,
        },
    };
    "example 4"
)]
// --------------------------------- example 5 ---------------------------------
#[test_case(
    vec![
        (Direction::Bid, 30, 25), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask,  5, 10), //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - 50% filled
    ],
    btree_map! {
        OrderId::new(!2) => 10,
        OrderId::new(!3) => 10,
        OrderId::new(6)  =>  5,
    },
    btree_map! {
        OrderId::new(!1) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(24),
            usdc::DENOM.clone()  => BalanceChange::Decreased(688), // -750 deposit, +62 refund
        },
        OrderId::new(!2) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        OrderId::new(!3) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        OrderId::new(4) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(273), // Receives two less due to fee
        },
        OrderId::new(5) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(273), // Receives two less due to fee
        },
        OrderId::new(6) => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(136), // refund: floor(5 * 27.5) = 137 minus 1 due to fee
        },
    };
    "example 5"
)]
fn dex_works(
    // A list of orders to submit: direction, price, amount.
    orders_to_submit: Vec<(Direction, u128, u128)>,
    // Orders that should remain not fully filled: order_id => remaining amount.
    remaining_orders: BTreeMap<OrderId, u128>,
    // Changes that should happen to the users' balances: order_id => denom => change.
    balance_changes: BTreeMap<OrderId, BTreeMap<Denom, BalanceChange>>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Register oracle price source for USDC and DANGO. Needed for volume tracking in cron_execute
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Find which accounts will submit the orders, so we can track their balances.
    let users_by_order_id = orders_to_submit
        .iter()
        .zip(accounts.users())
        .enumerate()
        .map(|(order_id, ((direction, ..), signer))| {
            let order_id = OrderId::new((order_id + 1) as u64);
            match direction {
                Direction::Bid => (!order_id, signer.address()),
                Direction::Ask => (order_id, signer.address()),
            }
        })
        .collect::<BTreeMap<_, _>>();

    // Track the users' balances.
    suite.balances().record_many(users_by_order_id.values());

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .zip(accounts.users_mut())
        .map(|((direction, price, amount), signer)| {
            let price = Udec128_24::new(price);
            let amount = Uint128::new(amount);

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(usdc::DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(dango::DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction,
                        amount: NonZero::new_unchecked(amount),
                        price: NonZero::new_unchecked(price),
                    }],
                    cancels: None,
                },
                funds,
            )?;

            signer.sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    // Make a block with the order submissions. Ensure all transactions were
    // successful.
    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    // Check the users' balances should have changed correctly.
    for (order_id, changes) in balance_changes {
        suite
            .balances()
            .should_change(&users_by_order_id[&order_id], changes);
    }

    // Check the remaining unfilled orders.
    let orders = suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .unwrap()
        .into_iter()
        .map(|(order_id, order)| (order_id, order.remaining.into_int().into_inner()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(orders, remaining_orders);
}

#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    None,
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!1) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(100),
            remaining: Udec128_6::new(100),
        },
    };
    "one submission no cancellations"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "one submission cancels one order"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    coins! { usdc::DENOM.clone() => 200 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(100),
            remaining: Udec128_6::new(100),
        },
    };
    "two submission cancels one order"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1), OrderId::new(!2)]))),
    coins! { usdc::DENOM.clone() => 200 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "two submission cancels both orders"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
    ],
    Some(CancelOrderRequest::All),
    coins! { usdc::DENOM.clone() => 200 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "two submission cancel all"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: NonZero::new_unchecked(Udec128_24::new(1)),
        },
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    coins! { usdc::DENOM.clone() => 199 },
    btree_map! {},
    btree_map! {}
    => panics "insufficient funds for batch updating orders";
    "two submission insufficient funds"
)]
fn submit_and_cancel_orders(
    submissions: Vec<CreateLimitOrderRequest>,
    cancellations: Option<CancelOrderRequest>,
    funds: Coins,
    expected_balance_changes: BTreeMap<Denom, BalanceChange>,
    expected_orders_after: BTreeMap<OrderId, OrderResponse>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Record the user's balance.
    suite.balances().record(&accounts.user1);

    // Add order to the order book.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: submissions,
                cancels: None,
            },
            funds,
        )
        .should_succeed();

    // Cancel the order.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![],
                cancels: cancellations,
            },
            coins! { dango::DENOM.clone() => 1 },
        )
        .should_succeed();

    // Check that the user balance has not changed.
    suite
        .balances()
        .should_change(&accounts.user1, expected_balance_changes);

    // Check that order does not exist.
    suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and(|orders| {
            assert_eq!(orders.len(), expected_orders_after.len());
            expected_orders_after
                .iter()
                .all(|(order_id, expected_order)| {
                    let actual_order = orders.get(order_id).unwrap();
                    actual_order.user == accounts.user1.address()
                        && actual_order.base_denom == expected_order.base_denom
                        && actual_order.quote_denom == expected_order.quote_denom
                        && actual_order.direction == expected_order.direction
                })
        });
}

#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(100),
            remaining: Udec128_6::new(100),
        },
    };
    "submit one order then cancel it and submit it again"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(50)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Increased(50) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(50),
            remaining: Udec128_6::new(50),
        },
    };
    "submit one order then cancel it and place a new order using half of the funds"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(200)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(200),
            remaining: Udec128_6::new(200),
        },
    };
    "submit one order then cancel it and place a new order using more funds"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(200)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(200),
            remaining: Udec128_6::new(200),
        },
    }
    => panics "insufficient funds";
    "submit one order then cancel it and place a new order with insufficient funds"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(150)),
        price: NonZero::new_unchecked(Udec128_24::new(1)),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(50) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128_24::new(1),
            amount: Uint128::new(150),
            remaining: Udec128_6::new(150),
        },
    };
    "submit one order then cancel it and place a new order excess funds are returned"
)]
fn submit_orders_then_cancel_and_submit_in_same_message(
    initial_orders: Vec<CreateLimitOrderRequest>,
    initial_funds: Coins,
    cancellations: Option<CancelOrderRequest>,
    new_orders: Vec<CreateLimitOrderRequest>,
    second_funds: Coins,
    expected_balance_changes: BTreeMap<Denom, BalanceChange>,
    expected_orders_after: BTreeMap<OrderId, OrderResponse>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Submit the initial orders
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: initial_orders,
                cancels: None,
            },
            initial_funds,
        )
        .should_succeed();

    // Record the user's balance
    suite.balances().record(&accounts.user1);

    // Cancel the initial orders
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: new_orders,
                cancels: cancellations,
            },
            second_funds,
        )
        .should_succeed();

    // Check that the user balance has changed
    suite
        .balances()
        .should_change(&accounts.user1, expected_balance_changes);

    // Check that the orders are as expected
    suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and(|orders| {
            assert_eq!(orders.len(), expected_orders_after.len());
            expected_orders_after
                .iter()
                .all(|(order_id, expected_order)| {
                    let actual_order = orders.get(order_id).unwrap();
                    actual_order.user == accounts.user1.address()
                        && actual_order.base_denom == expected_order.base_denom
                        && actual_order.quote_denom == expected_order.quote_denom
                        && actual_order.direction == expected_order.direction
                })
        });
}

#[test]
fn submit_and_cancel_order_in_same_block() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Record the user's balance
    suite.balances().record(&accounts.user1);

    // Build and sign a transaction with two messages: submit an order and cancel the order
    let submit_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates_market: vec![],
            creates_limit: vec![CreateLimitOrderRequest {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::new(100)),
                price: NonZero::new_unchecked(Udec128_24::new(1)),
            }],
            cancels: None,
        },
        coins! { usdc::DENOM.clone() => 100 },
    )
    .unwrap();

    let cancel_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates_market: vec![],
            creates_limit: vec![],
            cancels: Some(dex::CancelOrderRequest::Some(BTreeSet::from([
                OrderId::new(!1),
            ]))),
        },
        Coins::new(),
    )
    .unwrap();

    // Create a transaction with both messages
    let tx = accounts
        .user1
        .sign_transaction(
            NonEmpty::new_unchecked(vec![submit_order_msg, cancel_order_msg]),
            &suite.chain_id,
            100_000,
        )
        .unwrap();

    // Execute the transaction in a block
    suite
        .make_block(vec![tx])
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    // Check that the user balance has changed only by the gas fees
    suite.balances().should_change(&accounts.user1, btree_map! {
        dango::DENOM.clone() => BalanceChange::Unchanged
    });

    // Check that order does not exist
    suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and(BTreeMap::is_empty);
}

#[test_case(
    vec![
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (dango::DENOM.clone(), usdc::DENOM.clone()),
    None,
    None,
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        OrderId::new(!2) => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        OrderId::new(3)  => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
        OrderId::new(4)  => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
    };
    "dango/usdc no pagination"
)]
#[test_case(
    vec![
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (eth::DENOM.clone(), usdc::DENOM.clone()),
    None,
    None,
    btree_map! {
        OrderId::new(!5) => (Direction::Bid, Udec128::new(20), Uint128::new(10)),
        OrderId::new(6)  => (Direction::Ask, Udec128::new(25), Uint128::new(10)),
    };
    "eth/usdc no pagination"
)]
#[test_case(
    vec![
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (dango::DENOM.clone(), usdc::DENOM.clone()),
    None,
    Some(3),
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        OrderId::new(!2) => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        OrderId::new(3)  => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
    };
    "dango/usdc with limit no start after"
)]
#[test_case(
    vec![
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (dango::DENOM.clone(), usdc::DENOM.clone()),
    Some(OrderId::new(3)),
    None,
    btree_map! {
        OrderId::new(4) => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
    };
    "dango/usdc with start after"
)]
#[test_case(
    vec![
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((dango::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((eth::DENOM.clone(), usdc::DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (dango::DENOM.clone(), usdc::DENOM.clone()),
    Some(OrderId::new(!2)),
    Some(2),
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        OrderId::new(3)  => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
    };
    "dango/usdc with start after and limit"
)]
fn query_orders_by_pair(
    orders_to_submit: Vec<((Denom, Denom), Direction, u128, u128)>,
    (base_denom, quote_denom): (Denom, Denom),
    start_after: Option<OrderId>,
    limit: Option<u32>,
    expected_orders: BTreeMap<OrderId, (Direction, Udec128, Uint128)>,
) {
    // For this test, we need some ETH and USDC for user1.
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption {
        bridge_ops: |accounts| {
            vec![
                BridgeOp {
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::USDC_WARP,
                    },
                    amount: Uint128::new(100_000_000_000),
                    recipient: accounts.user1.address(),
                },
                BridgeOp {
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::WETH_WARP,
                    },
                    amount: Uint128::new(100_000_000_000),
                    recipient: accounts.user1.address(),
                },
            ]
        },
        ..Default::default()
    });

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .map(|((base_denom, quote_denom), direction, price, amount)| {
            let price = Udec128_24::new(price);
            let amount = Uint128::new(amount);

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(quote_denom.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(base_denom.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom,
                        quote_denom,
                        direction,
                        amount: NonZero::new_unchecked(amount),
                        price: NonZero::new_unchecked(price),
                    }],
                    cancels: None,
                },
                funds,
            )?;

            accounts.user1.sign_transaction(
                NonEmpty::new_unchecked(vec![msg]),
                &suite.chain_id,
                100_000,
            )
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    // Make a block with the order submissions. Ensure all transactions were
    // successful.
    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    suite
        .query_wasm_smart(contracts.dex, QueryOrdersByPairRequest {
            base_denom,
            quote_denom,
            start_after,
            limit,
        })
        .should_succeed_and(|orders| {
            assert_eq!(orders.len(), expected_orders.len());
            expected_orders
                .iter()
                .all(|(order_id, (direction, price, amount))| {
                    let queried_order = orders.get(order_id).unwrap();
                    queried_order.direction == *direction
                        && queried_order.price == price.convert_precision().unwrap()
                        && queried_order.amount == *amount
                        && queried_order.remaining == amount.checked_into_dec().unwrap()
                        && queried_order.user == accounts.user1.address()
                })
        });
}

#[test]
fn only_owner_can_create_passive_pool() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let lp_denom = Denom::try_from("dex/pool/xrp/usdc").unwrap();

    // Attempt to create pair as non-owner. Should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: xrp::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params: PairParams {
                    lp_denom: lp_denom.clone(),
                    pool_type: PassiveLiquidity::Xyk {
                        order_spacing: Udec128::new_bps(1),
                        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                },
            }])),
            Coins::new(),
        )
        .should_fail_with_error("you don't have the right, O you don't have the right");

    // Attempt to create pair as owner. Should succeed.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: xrp::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params: PairParams {
                    lp_denom: lp_denom.clone(),
                    pool_type: PassiveLiquidity::Xyk {
                        order_spacing: Udec128::new_bps(1),
                        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                },
            }])),
            Coins::new(),
        )
        .should_succeed();
}

#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 100,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(100_001_000)
    ; "provision at pool ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(50_000_500)
    ; "provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(72_965_967)
    ; "provision at different ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 100,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric {
        order_spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(300_001_000)
    ; "geometric pool provision at pool ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric {
        order_spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(150_000_500)
    ; "geometric pool provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric {
        order_spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(249_909_923)
    ; "geometric pool provision at different ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 100,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric {
        order_spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(199_900_665)
    ; "geometric pool provision at different ratio 2"
)]
fn provide_liquidity(
    provision: Coins,
    swap_fee: Udec128,
    pool_type: PassiveLiquidity,
    oracle_prices: Vec<(Denom, Udec128)>,
    expected_lp_balance: Uint128,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let lp_denom = Denom::try_from("dex/pool/dango/usdc").unwrap();

    // Owner first provides some initial liquidity.
    let initial_reserves = coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone()  => 100,
    };

    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            // Update pair params
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee),
                            pool_type,
                        },
                    }])),
                    Coins::new(),
                )
                .should_succeed();
            true
        });

    // Register the oracle prices
    for (denom, price) in oracle_prices {
        suite
            .execute(
                &mut accounts.owner,
                contracts.oracle,
                &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                    denom => PriceSource::Fixed {
                        humanized_price: price,
                        precision: 6,
                        timestamp: Timestamp::from_seconds(1730802926),
                    },
                }),
                Coins::new(),
            )
            .should_succeed();
    }

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            initial_reserves.clone(),
        )
        .should_succeed();

    // Record the users initial balances.
    suite.balances().record_many(accounts.users());

    // Execute all the provisions.
    let mut expected_pool_balances = initial_reserves.clone();

    // record the dex balance
    suite.balances().record(&contracts.dex);

    // Execute provide liquidity
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            provision.clone(),
        )
        .should_succeed();

    // Ensure that the dex balance has increased by the expected amount.
    suite.balances().should_change(
        &contracts.dex,
        balance_changes_from_coins(provision.clone(), Coins::new()),
    );

    // Ensure user's balance has decreased by the expected amount and that
    // LP tokens have been minted.
    suite.balances().should_change(
        &accounts.user1,
        balance_changes_from_coins(
            coins! { lp_denom.clone() => expected_lp_balance },
            provision.clone(),
        ),
    );

    // Check that the reserves in pool object were updated correctly.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryReserveRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(
            expected_pool_balances
                .insert_many(provision)
                .unwrap()
                .take_pair((dango::DENOM.clone(), usdc::DENOM.clone()))
                .unwrap(),
        );
}

#[test]
fn provide_liquidity_to_geometric_pool_should_fail_without_oracle_price() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Update pair params
    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            // Update pair params
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            swap_fee_rate: pair_params.swap_fee_rate,
                            pool_type: PassiveLiquidity::Geometric {
                                order_spacing: Udec128::ONE,
                                ratio: Bounded::new_unchecked(Udec128::ONE),
                            },
                        },
                    }])),
                    Coins::new(),
                )
                .should_succeed();
            true
        });

    // Since there is no oracle price, liquidity provision should fail.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            coins! {
                dango::DENOM.clone() => 100_000,
                usdc::DENOM.clone() => 100_000,
            },
        )
        .should_fail_with_error(StdError::data_not_found::<(PrecisionlessPrice, u64)>(
            PRICES.path(USDC_USD_ID).storage_key(),
        ));
}

#[test_case(
    Uint128::new(100_001_000),
    Udec128::new_permille(5),
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone()  => 100,
    };
    "withdrawa all"
)]
#[test_case(
    Uint128::new(50_000_500),
    Udec128::new_permille(5),
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone()  => 50,
    };
    "withdraw half"
)]
fn withdraw_liquidity(lp_burn_amount: Uint128, swap_fee: Udec128, expected_funds_returned: Coins) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let lp_denom = Denom::try_from("dex/pool/dango/usdc").unwrap();

    // Owner first provides some initial liquidity.
    let initial_reserves = coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone()  => 100,
    };

    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            // Update pair params
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee),
                            pool_type: pair_params.pool_type.clone(),
                        },
                    }])),
                    Coins::new(),
                )
                .should_succeed();
            true
        });

    // Register the oracle prices
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Owner provides some initial liquidity.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            initial_reserves.clone(),
        )
        .should_succeed();

    // User provides some liquidity.
    let provided_funds = coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 100,
    };
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            provided_funds.clone(),
        )
        .should_succeed();

    // record user and dex balances
    suite
        .balances()
        .record_many([&accounts.user1.address(), &contracts.dex]);

    // withdraw liquidity
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::WithdrawLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            coins! { lp_denom.clone() => lp_burn_amount },
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    suite.balances().should_change(
        &accounts.user1,
        balance_changes_from_coins(
            expected_funds_returned.clone(),
            coins! { lp_denom.clone() => lp_burn_amount },
        ),
    );

    // Assert that the dex balance has decreased by the expected amount.
    suite.balances().should_change(
        &contracts.dex,
        balance_changes_from_coins(Coins::new(), expected_funds_returned.clone()),
    );

    // Assert pool reserves are updated correctly
    suite
        .query_wasm_smart(contracts.dex, dango_types::dex::QueryReserveRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal({
            initial_reserves
                .clone()
                .insert_many(provided_funds)
                .unwrap()
                .deduct_many(expected_funds_returned)
                .unwrap()
                .take_pair((dango::DENOM.clone(), usdc::DENOM.clone()))
                .unwrap()
        });
}

#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
    },
    None,
    coins! {
        usdc::DENOM.clone() => 495510,
    },
    coins! {
        usdc::DENOM.clone() => 1990,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 1000000,
            usdc::DENOM.clone() => 1000000 - 497500,
        },
    };
    "1:1 pool no swap fee one step route input 100% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    coins! {
        dango::DENOM.clone() => 500000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
    },
    None,
    coins! {
        usdc::DENOM.clone() => 330339,
    },
    coins! {
        usdc::DENOM.clone() => 1327,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 500000,
            usdc::DENOM.clone() => 1000000 - 331666,
        },
    };
    "1:1 pool no swap fee one step route input 50% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    coins! {
        dango::DENOM.clone() => 331666,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
    },
    None,
    coins! {
        usdc::DENOM.clone() => 246822,
    },
    coins! {
        usdc::DENOM.clone() => 992,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 331666,
            usdc::DENOM.clone() => 1000000 - 247814,
        },
    };
    "1:1 pool no swap fee one step route input 33% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            eth::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![
        PairId {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        },
        PairId {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        }
    ],
    coins! {
        dango::DENOM.clone() => 500000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
        (eth::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
    },
    None,
    coins! {
        eth::DENOM.clone() => 246822,
    },
    coins! {
        eth::DENOM.clone() => 992,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 500000,
            usdc::DENOM.clone() => 1000000 - 331666,
        },
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            eth::DENOM.clone() => 1000000 - 247814,
            usdc::DENOM.clone() => 1000000 + 331666,
        },
    };
    "1:1 pools 0.5% swap fee input 100% of pool liquidity two step route"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
    },
    Some(500000u128.into()),
    coins! {
        usdc::DENOM.clone() => 500000,
    },
    coins! {
        usdc::DENOM.clone() => 1990,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 500000,
        },
    } => panics "output amount is below the minimum: 495510 < 500000" ;
    "1:1 pool no swap fee one step route input 100% of pool liquidity output is less than minimum output"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(5),
    },
    Some(495510u128.into()),
    coins! {
        usdc::DENOM.clone() => 495510,
    },
    coins! {
        usdc::DENOM.clone() => 1990,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 1000000 - 497500,
        },
    };
    "1:1 pool no swap fee one step route input 100% of pool liquidity output is not less than minimum output"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_bps(1),
    },
    None,
    coins! {
        usdc::DENOM.clone() => 497950,
    },
    coins! {
        usdc::DENOM.clone() => 2000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 1000000 - 499950,
        },
    };
    "1:1 pool 0.01% swap fee one step route input 100% of pool liquidity"
)]
fn swap_exact_amount_in(
    pool_reserves: BTreeMap<(Denom, Denom), Coins>,
    route: Vec<PairId>,
    swap_funds: Coins,
    swap_fee_rates: BTreeMap<(Denom, Denom), Udec128>,
    minimum_output: Option<Uint128>,
    expected_out: Coins,
    expected_protocol_fee: Coins,
    expected_pool_reserves_after: BTreeMap<(Denom, Denom), Coins>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    for ((base_denom, quote_denom), swap_fee_rate) in swap_fee_rates {
        suite
            .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
            })
            .should_succeed_and(|pair_params: &PairParams| {
                // Update pair params
                suite
                    .execute(
                        &mut accounts.owner,
                        contracts.dex,
                        &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![
                            PairUpdate {
                                base_denom: base_denom.clone(),
                                quote_denom: quote_denom.clone(),
                                params: PairParams {
                                    lp_denom: pair_params.lp_denom.clone(),
                                    swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                                    pool_type: pair_params.pool_type.clone(),
                                },
                            },
                        ])),
                        Coins::new(),
                    )
                    .should_succeed();
                true
            });
    }

    // Provide liquidity with owner account
    for ((base_denom, quote_denom), reserve) in pool_reserves {
        suite
            .execute(
                &mut accounts.owner,
                contracts.dex,
                &dex::ExecuteMsg::ProvideLiquidity {
                    base_denom: base_denom.clone(),
                    quote_denom: quote_denom.clone(),
                },
                reserve.clone(),
            )
            .should_succeed();
    }

    // Record user and dex balances
    suite
        .balances()
        .record_many([&accounts.user1.address(), &contracts.dex, &contracts.taxman]);

    // User swaps
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountIn {
                route: MaxLength::new_unchecked(UniqueVec::new_unchecked(route)),
                minimum_output,
            },
            swap_funds.clone(),
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    suite.balances().should_change(
        &accounts.user1,
        balance_changes_from_coins(expected_out.clone(), swap_funds.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    suite.balances().should_change(
        &contracts.dex,
        balance_changes_from_coins(
            swap_funds.clone(),
            expected_out
                .clone()
                .insert_many(expected_protocol_fee.clone())
                .unwrap()
                .clone(),
        ),
    );

    // Assert that the expected protocol fee was transferred to the taxman.
    suite.balances().should_change(
        &contracts.taxman,
        balance_changes_from_coins(expected_protocol_fee.clone(), Coins::new()),
    );

    // Query pools and assert that the reserves are updated correctly
    for ((base_denom, quote_denom), expected_reserve) in expected_pool_reserves_after {
        suite
            .query_wasm_smart(contracts.dex, QueryReserveRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
            })
            .should_succeed_and_equal(CoinPair::try_from(expected_reserve).unwrap());
    }
}

#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 498000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1002006,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1002006).unwrap(),
    Coin::new(usdc::DENOM.clone(), 2000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 1002006,
            usdc::DENOM.clone() => 1000000 - 500000,
        },
    };
    "1:1 pool 0.1% swap fee one step route output 50% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 331999).unwrap(),
    coins! {
        dango::DENOM.clone() => 500751,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 500751).unwrap(),
    Coin::new(usdc::DENOM.clone(), 1334).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 500751,
            usdc::DENOM.clone() => 1000000 - 333333,
        },
    };
    "1:1 pool 0.1% swap fee one step route output 33% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 249000).unwrap(),
    coins! {
        dango::DENOM.clone() => 333779,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 333779).unwrap(),
    Coin::new(usdc::DENOM.clone(), 1000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 333779,
            usdc::DENOM.clone() => 1000000 - 250000,
        },
    };
    "1:1 pool 0.1% swap fee one step route output 25% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 996000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1000000).unwrap(),
    Coin::new(usdc::DENOM.clone(), 4000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 500000,
        },
    }
    => panics "insufficient liquidity" ;
    "1:1 pool no swap fee one step route output 100% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 498000).unwrap(),
    coins! {
        dango::DENOM.clone() => 999999,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1000000).unwrap(),
    Coin::new(usdc::DENOM.clone(), 2000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 500000,
        },
    }
    => panics "insufficient input for swap" ;
    "1:1 pool no swap fee one step route output 50% of pool liquidity insufficient funds"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 498000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1100000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1002006).unwrap(),
    Coin::new(usdc::DENOM.clone(), 2000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 1002006,
            usdc::DENOM.clone() => 1000000 - 500000,
        },
    };
    "1:1 pool 0.1% swap fee one step route output 50% of pool liquidity excessive funds returned"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            eth::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![
        PairId {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        },
        PairId {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        },
    ],
    Coin::new(eth::DENOM.clone(), 249000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
        (eth::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 501758).unwrap(),
    Coin::new(eth::DENOM.clone(), 1000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 501758,
            usdc::DENOM.clone() => 1000000 - 333779,
        },
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            eth::DENOM.clone() => 1000000 - 250000,
            usdc::DENOM.clone() => 1000000 + 333779,
        },
    };
    "1:1 pool 0.1% swap fee two step route output 25% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 497950).unwrap(),
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_bps(1),
    },
    Coin::new(dango::DENOM.clone(), 1000000).unwrap(),
    Coin::new(usdc::DENOM.clone(), 2000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 1000000 - 499950,
        },
    };
    "1:1 pool 0.01% swap fee one step route output 49.995% of pool liquidity"
)]
fn swap_exact_amount_out(
    pool_reserves: BTreeMap<(Denom, Denom), Coins>,
    route: Vec<PairId>,
    exact_out: Coin,
    swap_funds: Coins,
    swap_fee_rates: BTreeMap<(Denom, Denom), Udec128>,
    expected_in: Coin,
    expected_protocol_fee: Coin,
    expected_pool_reserves_after: BTreeMap<(Denom, Denom), Coins>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Update the pairs with the new swap fee rates.
    for ((base_denom, quote_denom), swap_fee_rate) in swap_fee_rates {
        let mut params = suite
            .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
            })
            .should_succeed();

        if params.swap_fee_rate.into_inner() != swap_fee_rate {
            params.swap_fee_rate = Bounded::new_unchecked(swap_fee_rate);

            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: base_denom.clone(),
                        quote_denom: quote_denom.clone(),
                        params,
                    }])),
                    Coins::new(),
                )
                .should_succeed();
        }
    }

    // Provide liquidity with owner account
    for ((base_denom, quote_denom), reserve) in pool_reserves {
        suite
            .execute(
                &mut accounts.owner,
                contracts.dex,
                &dex::ExecuteMsg::ProvideLiquidity {
                    base_denom: base_denom.clone(),
                    quote_denom: quote_denom.clone(),
                },
                reserve.clone(),
            )
            .should_succeed();
    }

    // Record user and dex balances
    suite
        .balances()
        .record_many([&accounts.user1.address(), &contracts.dex, &contracts.taxman]);

    // User swaps
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountOut {
                route: MaxLength::new_unchecked(UniqueVec::new_unchecked(route)),
                output: NonZero::new(exact_out.clone()).unwrap(),
            },
            swap_funds.clone(),
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    let mut expected_out_coins: Coins = vec![exact_out].try_into().unwrap();
    let expected_in_coins: Coins = vec![expected_in].try_into().unwrap();
    suite.balances().should_change(
        &accounts.user1,
        balance_changes_from_coins(expected_out_coins.clone(), expected_in_coins.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    expected_out_coins
        .insert(expected_protocol_fee.clone())
        .unwrap();
    suite.balances().should_change(
        &contracts.dex,
        balance_changes_from_coins(expected_in_coins.clone(), expected_out_coins.clone()),
    );

    // Assert that the taxman balance has changed by the expected amount.
    suite.balances().should_change(
        &contracts.taxman,
        balance_changes_from_coins(
            vec![expected_protocol_fee].try_into().unwrap(),
            Coins::new(),
        ),
    );

    // Query pools and assert that the reserves are updated correctly
    for ((base_denom, quote_denom), expected_reserve) in expected_pool_reserves_after {
        suite
            .query_wasm_smart(contracts.dex, QueryReserveRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
            })
            .should_succeed_and(|reserve: &CoinPair| {
                reserve.clone() == CoinPair::try_from(expected_reserve).unwrap()
            });
    }
}

#[test]
fn geometric_pool_swaps_fail_without_oracle_price() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Provide liquidity to pair before changing the pool type since XYK does not require an oracle price
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            coins! {
                dango::DENOM.clone() => 1000000,
                usdc::DENOM.clone() => 1000000,
            },
        )
        .should_succeed();

    // Update pair params to change pool type to geometric
    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            // Update pair params
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            swap_fee_rate: pair_params.swap_fee_rate,
                            pool_type: PassiveLiquidity::Geometric {
                                order_spacing: Udec128::ONE,
                                ratio: Bounded::new_unchecked(Udec128::ONE),
                            },
                        },
                    }])),
                    Coins::new(),
                )
                .should_succeed();
            true
        });

    // Ensure swap exact amount in fails
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountIn {
                route: MaxLength::new_unchecked(UniqueVec::new_unchecked(vec![PairId {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                }])),
                minimum_output: None,
            },
            coins! {
                dango::DENOM.clone() => 1000000,
            },
        )
        .should_fail_with_error(StdError::data_not_found::<PriceSource>(
            PRICE_SOURCES.path(&dango::DENOM).storage_key(),
        ));

    // Ensure swap exact amount out fails
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountOut {
                route: MaxLength::new_unchecked(UniqueVec::new_unchecked(vec![PairId {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                }])),
                output: NonZero::new(Coin::new(dango::DENOM.clone(), 50000).unwrap()).unwrap(),
            },
            coins! {
                usdc::DENOM.clone() => 1000000,
            },
        )
        .should_fail_with_error(StdError::data_not_found::<PriceSource>(
            PRICE_SOURCES.path(&dango::DENOM).storage_key(),
        ));
}

#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::from(49751)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(20100)),
            },
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 49751 * 201,
        },
    ],
    btree_map! {
        OrderId::new(!1) => (Udec128::new_percent(20100), Udec128::new(49751), Direction::Bid),
    },
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000,
            usdc::DENOM.clone() => 200 * 10000000,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Unchanged,
        usdc::DENOM.clone() => BalanceChange::Increased(49751 * 201),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone() => BalanceChange::Decreased(49751 * 201),
        },
    ];
    "xyk pool balance 1:200 tick size 1 one percent fee no matching orders"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::from(49751)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(20100)),
            },
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 49751 * 201,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 - 49751,
            usdc::DENOM.clone() => 200 * 10000000 + 49751 * 201,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(49751),
        usdc::DENOM.clone() => BalanceChange::Increased(49751 * 201),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Increased(49751),
            usdc::DENOM.clone() => BalanceChange::Decreased(49751 * 201),
        },
    ];
    "xyk pool balance 1:200 tick size 1 no fee user bid order exactly matches passive order"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::from(47783)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(20200)),
            },
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 47783 * 202,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 - 47783,
            usdc::DENOM.clone() => 200 * 10000000 + 47783 * 202,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(47783),
        usdc::DENOM.clone() => BalanceChange::Increased(47783 * 202),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Increased(47783),
            usdc::DENOM.clone() => BalanceChange::Decreased(47783 * 202),
        },
    ];
    "xyk pool balance 1:200 tick size 1 one percent fee user bid order partially fills passive order"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::from(157784)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(20300)),
            },
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 157784 * 203,
        },
    ],
    btree_map! {
        OrderId::new(!1) => (Udec128::new_percent(20300), Udec128::new(10000), Direction::Bid),
    },
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 - 147784,
            usdc::DENOM.clone() => 200 * 10000000 + 147784 * 203,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(147784),
        usdc::DENOM.clone() => BalanceChange::Increased(157784 * 203),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Increased(147784),
            usdc::DENOM.clone() => BalanceChange::Decreased(157784 * 203),
        }
    ];
    "xyk pool balance 1:200 tick size 1 one percent fee user bid order fully fills passive order with amount remaining after"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Ask,
                amount: NonZero::new_unchecked(Uint128::from(50251)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(19900)),
            },
        ],
    ],
    vec![
        coins! {
            eth::DENOM.clone() => 50251,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 + 50251,
            usdc::DENOM.clone() => 200 * 10000000 - 50251 * 199,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(50251),
        usdc::DENOM.clone() => BalanceChange::Decreased(50251 * 199),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Decreased(50251),
            usdc::DENOM.clone() => BalanceChange::Increased(50251 * 199),
        },
    ];
    "xyk pool balance 1:200 tick size 1 no fee user ask order exactly matches passive order"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Ask,
                amount: NonZero::new_unchecked(Uint128::from(30000)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(19900)),
            },
        ],
    ],
    vec![
        coins! {
            eth::DENOM.clone() => 30000,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 + 30000,
            usdc::DENOM.clone() => 200 * 10000000 - 30000 * 199,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(30000),
        usdc::DENOM.clone() => BalanceChange::Decreased(30000 * 199),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Decreased(30000),
            usdc::DENOM.clone() => BalanceChange::Increased(30000 * 199),
        },
    ];
    "xyk pool balance 1:200 tick size 1 no fee user ask order partially fills passive order"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Ask,
                amount: NonZero::new_unchecked(Uint128::from(60251)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(19900)),
            },
        ],
    ],
    vec![
        coins! {
            eth::DENOM.clone() => 60251,
        },
    ],
    btree_map! {
        OrderId::new(1) => (Udec128::new_percent(19900), Udec128::new(10000), Direction::Ask),
    },
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 + 50251,
            usdc::DENOM.clone() => 200 * 10000000 - 50251 * 199,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(60251),
        usdc::DENOM.clone() => BalanceChange::Decreased(50251 * 199),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Decreased(60251),
            usdc::DENOM.clone() => BalanceChange::Increased(50251 * 199),
        },
    ];
    "xyk pool balance 1:200 tick size 1 no fee user ask order fully fills passive order with amount remaining after"
)]
#[test_case(
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
    },
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                direction: Direction::Ask,
                amount: NonZero::new_unchecked(Uint128::from(162284)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(19800)),
            },
        ],
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::from(157784)),
                price: NonZero::new_unchecked(Udec128_24::new_percent(20200)),
            },
        ],
    ],
    vec![
        coins! {
            eth::DENOM.clone() => 162284,
        },
        coins! {
            usdc::DENOM.clone() => 157784 * 203,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coin_pair! {
            eth::DENOM.clone() => 10000000 + 4500, // only the remaining amount of the ask order traded against the passive pool
            usdc::DENOM.clone() => 200 * 10000000 - 4500 * 198,
        },
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(162284 - 157784),
        usdc::DENOM.clone() => BalanceChange::Decreased(4500 * 198),
    },
    vec![
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Decreased(162284),
            usdc::DENOM.clone() => BalanceChange::Increased(162284 * 198),
        },
        btree_map! {
            eth::DENOM.clone() => BalanceChange::Increased(157784),
            usdc::DENOM.clone() => BalanceChange::Decreased(157784 * 198),
        },
    ];
    "xyk pool balance 1:200 tick size 1 one percent fee three users with multiple orders"
)]
fn curve_on_orderbook(
    pool_type: PassiveLiquidity,
    swap_fee_rate: Udec128,
    pool_liquidity: Coins,
    orders: Vec<Vec<CreateLimitOrderRequest>>,
    order_creation_funds: Vec<Coins>,
    expected_orders_after_clearing: BTreeMap<OrderId, (Udec128, Udec128, Direction)>,
    expected_reserves_after_clearing: BTreeMap<(Denom, Denom), CoinPair>,
    expected_dex_balance_changes: BTreeMap<Denom, BalanceChange>,
    expected_user_balance_changes: Vec<BTreeMap<Denom, BalanceChange>>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set maker and taker fee rates to 0 for simplicity
    let mut app_config: AppConfig = suite.query_app_config().unwrap();
    app_config.maker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    app_config.taker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    suite
        .configure(
            &mut accounts.owner, // Must be the chain owner
            None,                // No chain config update
            Some(app_config),    // App config update
        )
        .should_succeed();

    // Update pair params
    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            // Provide liquidity with owner account
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            pool_type,
                            swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                        },
                    }])),
                    pool_liquidity.clone(),
                )
                .should_succeed();
            true
        });

    // Register oracle price source for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Register oracle price source for ETH
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                eth::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::new_percent(2000),
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Provide liquidity with owner account
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            },
            pool_liquidity.clone(),
        )
        .should_succeed();

    // Record dex and user balances
    suite.balances().record(&contracts.dex.address());
    suite.balances().record_many(accounts.users());

    // Create txs for all the orders from all users.
    let txs = accounts
        .users_mut()
        .zip(orders)
        .zip(order_creation_funds)
        .map(|((user, orders), order_creation_funds)| {
            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: Vec::new(),
                    creates_limit: orders,
                    cancels: None,
                },
                order_creation_funds,
            )?;

            user.sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    // Make a block with the order submissions. Ensure all transactions were
    // successful.
    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    // Assert that user balances have changed as expected
    for (user, expected_user_balance_change) in accounts.users().zip(expected_user_balance_changes)
    {
        suite
            .balances()
            .should_change(&user.address(), expected_user_balance_change);
    }

    // Assert that dex balances have changed as expected
    suite
        .balances()
        .should_change(&contracts.dex.address(), expected_dex_balance_changes);

    // Assert that reserves have changed as expected
    for ((base_denom, quote_denom), expected_reserve) in expected_reserves_after_clearing {
        suite
            .query_wasm_smart(contracts.dex, QueryReserveRequest {
                base_denom: base_denom.clone(),
                quote_denom: quote_denom.clone(),
            })
            .should_succeed_and_equal(expected_reserve);
    }

    // Assert that the order book contains the expected orders
    suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and(|orders| {
            assert_eq!(orders.len(), expected_orders_after_clearing.len());
            for (order_id, (price, remaining, direction)) in expected_orders_after_clearing {
                let order = orders.get(&order_id).unwrap();
                assert_eq!(order.price, price.convert_precision().unwrap());
                assert_eq!(order.remaining, remaining.convert_precision().unwrap());
                assert_eq!(order.direction, direction);
            }
            true
        });
}

fn balance_changes_from_coins(
    increases: Coins,
    decreases: Coins,
) -> BTreeMap<Denom, BalanceChange> {
    increases
        .into_iter()
        .map(|Coin { denom, amount }| {
            (denom.clone(), BalanceChange::Increased(amount.into_inner()))
        })
        .chain(decreases.into_iter().map(|Coin { denom, amount }| {
            (denom.clone(), BalanceChange::Decreased(amount.into_inner()))
        }))
        .collect()
}

#[test]
fn volume_tracking_works() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Register oracle price source for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Register oracle price source for DANGO
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    let mut user1_addr_1 = accounts.user1;
    let mut user1_addr_2 = user1_addr_1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Spot(Params::new(user1_addr_1.username.clone())),
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap();

    let mut user2_addr_1 = accounts.user2;
    let mut user2_addr_2 = user2_addr_1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Spot(Params::new(user2_addr_1.username.clone())),
            Coins::one(dango::DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap();

    // Query volumes before, should be 0
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Submit a new order with user1 address 1
    suite
        .execute(
            &mut user1_addr_1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // User2 submit an opposite matching order with address 1
    suite
        .execute(
            &mut user2_addr_1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            Coins::one(dango::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Get timestamp after trade
    let timestamp_after_first_trade = suite.block.timestamp;

    // Query the volume for username user1, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    // Query the volume for username user2, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user2_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    // Query the volume for user1 address 1, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    // Query the volume for user2 address 1, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    // Query the volume for user1 address 2, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Query the volume for user2 address 2, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Submit a new order with user1 address 2
    suite
        .execute(
            &mut user1_addr_2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Submit a new opposite matching order with user2 address 2
    suite
        .execute(
            &mut user2_addr_2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            Coins::one(dango::DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Query the volume for username user1, should be 200
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(200));

    // Query the volume for username user2, should be 200
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user2_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(200));

    // Query the volume for all addresses, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(100));

    // Query the volume for both usernames since timestamp after first trade, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user2_addr_1.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::new(100));

    // Query the volume for both users address 1 since timestamp after first trade, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Query the volume for both users address 2 since timestamp after first trade, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::new(100));
}

#[test]
fn volume_tracking_works_with_multiple_orders_from_same_user() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Register oracle price source for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Register oracle price source for DANGO
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Register oracle price source for ETH
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                eth::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::from_str("85248.71").unwrap(),
                    precision: 8,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Submit two orders for DANGO/USDC and one for ETH/USDC with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: NonZero::new_unchecked(Udec128_24::new(1)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: NonZero::new_unchecked(Udec128_24::from_str("1.01").unwrap()),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: NonZero::new_unchecked(Udec128_24::from_str("852.485845").unwrap()),
                    },
                ],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 301_000_000).unwrap(),
        )
        .should_succeed();

    // Submit matching orders with user2
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(200_000_000)),
                        price: NonZero::new_unchecked(Udec128_24::new(1)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: NonZero::new_unchecked(Udec128_24::from_str("852.485845").unwrap()),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => 200_000_000,
                eth::DENOM.clone() => 117304,
            },
        )
        .should_succeed();

    // Get timestamp after trade
    let timestamp_after_first_trade = suite.block.timestamp;

    // Query the volume for username user1, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("300.000146").unwrap());

    // Query the volume for username user2, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("300.000146").unwrap());

    // Query the volume for user1 address, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("300.000146").unwrap());

    // Query the volume for user2 address, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("300.000146").unwrap());

    // Query the volume for both usernames since timestamp after first trade, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user1.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Submit new orders with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: NonZero::new_unchecked(Udec128_24::new(1)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: NonZero::new_unchecked(Udec128_24::from_str("1.01").unwrap()),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: NonZero::new_unchecked(Udec128_24::from_str("852.485845").unwrap()),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: NonZero::new_unchecked(Udec128_24::from_str("937.7344336").unwrap()),
                    },
                ],
                cancels: None,
            },
            coins! {
                usdc::DENOM.clone() => 411_000_000,
            },
        )
        .should_succeed();

    // Submit matching orders with user2
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(300_000_000)),
                        price: NonZero::new_unchecked(Udec128_24::new(1)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(117304 * 2)),
                        price: NonZero::new_unchecked(
                            Udec128_24::from_str("85248.71")
                                .unwrap()
                                .checked_inv()
                                .unwrap(),
                        ),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => 300_000_000,
                eth::DENOM.clone() => 117304 * 2,
            },
        )
        .should_succeed();

    // Get timestamp after second trade
    let timestamp_after_second_trade = suite.block.timestamp;

    // Query the volume for username user1, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("700.000438").unwrap());

    // Query the volume for username user2, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("700.000439").unwrap());

    // Query the volume for user1 address, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user1.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("700.000438").unwrap());

    // Query the volume for user2 address, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user2.address(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("700.000439").unwrap());

    // Query the volume for both usernames since timestamp after second trade, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user1.username.clone(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Query the volume for both addresses since timestamp after the first trade, should be 400
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user1.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::from_str("400.000292").unwrap());
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user2.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::from_str("400.000293").unwrap());
}

#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Unchanged,
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone() => BalanceChange::Unchanged,
        },
    },
    btree_map! {
        OrderId::new(1) => (Direction::Ask, Udec128::new(1), Uint128::new(100_000_000), Uint128::new(100_000_000), 0),
    };
    "One limit ask, one market ask, no match"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            dango::DENOM.clone() => BalanceChange::Unchanged,
        },
        1 => btree_map! {
            usdc::DENOM.clone() => BalanceChange::Unchanged,
            dango::DENOM.clone() => BalanceChange::Unchanged,
        },
    },
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new(1), Uint128::new(100_000_000), Uint128::new(100_000_000), 0),
    };
    "One limit bid, one market bid, no match"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit bid, one market ask, same size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(2)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(200_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(200_000_000),
        },
    },
    BTreeMap::new();
    "One limit bid price 2.0, one market ask, same size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask, one market bid, same size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(2)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(200_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(200_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(200_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask price 2.0, one market bid, same size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(150_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    btree_map! {
        OrderId::new(1) => (Direction::Ask, Udec128::new(1), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
    };
    "One limit ask price 1.0, one market bid, limit order larger size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(2)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(200_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(150_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(200_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(200_000_000),
        },
    },
    btree_map! {
        OrderId::new(1) => (Direction::Ask, Udec128::new(2), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
    };
    "One limit ask price 2.0, one market bid, limit order larger size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(150_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
    },
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new(1), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
    };
    "One limit bid price 1.0, one market ask, limit order larger size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(2)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 300_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(300_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(200_000_000),
        },
    },
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new(2), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
    };
    "One limit bid price 2.0, one market ask, limit order larger size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask, one market bid, limit order smaller size, no fees, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new_percent(200)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::new_percent(99),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(50_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(125_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(150_000_000),
        },
    },
    btree_map! {
        OrderId::new(2) => (Direction::Ask, Udec128::new_percent(200), Uint128::new(100_000_000), Uint128::new(75_000_000), 1),
    };
    "Two limit asks different prices, one market bid, second limit partially filled, no fees, slippage not exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new_percent(200)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(200_000_000)),
                    max_slippage: Udec128::new_percent(20),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(50_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(125_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(150_000_000),
        },
    },
    btree_map! {
        OrderId::new(2) => (Direction::Ask, Udec128::new_percent(200), Uint128::new(100_000_000), Uint128::new(75_000_000), 1),
    };
    "Two limit asks different prices, one market bid, second limit partially filled, no fees, slippage exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new_percent(50)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 50_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::new_percent(99),
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(50_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(50_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(150_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(125_000_000),
        },
    },
    btree_map! {
        OrderId::new(!2) => (Direction::Bid, Udec128::new_percent(50), Uint128::new(100_000_000), Uint128::new(50_000_000), 1),
    };
    "Two limit bids different prices, one market ask, second limit partially filled, no fees, slippage not exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new_percent(10)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 10_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::new_percent(10),
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(12_500_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(10_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(112_500_000),
            usdc::DENOM.clone() => BalanceChange::Increased(101_250_000),
        },
    },
    btree_map! {
        OrderId::new(!2) => (Direction::Bid, Udec128::new_percent(10), Uint128::new(100_000_000), Uint128::new(87_500_000), 1),
    };
    "Two limit bids different prices, one market ask, second limit partially filled, no fees, slippage exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    max_slippage: Udec128::new_percent(99),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 50_000_000,
            },
        ),
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(30_000_000)),
                    max_slippage: Udec128::new_percent(99),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 30_000_000,
            },
        ),

    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(80_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(50_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(50_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(30_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(30_000_000),
        },
    },
    btree_map! {
        OrderId::new(1) => (Direction::Ask, Udec128::new_percent(100), Uint128::new(100_000_000), Uint128::new(20_000_000), 0),
    };
    "One limit ask, two market bids, no fees, slippage not exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    max_slippage: Udec128::new_percent(99),
                },
            ],
            coins! {
                dango::DENOM.clone() => 50_000_000,
            },
        ),
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(30_000_000)),
                    max_slippage: Udec128::new_percent(99),
                },
            ],
            coins! {
                dango::DENOM.clone() => 30_000_000,
            },
        ),

    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(80_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(50_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(50_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(30_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(30_000_000),
        },
    },
    btree_map! {
        OrderId::new(!1) => (Direction::Bid, Udec128::new_percent(100), Uint128::new(100_000_000), Uint128::new(20_000_000), 0),
    };
    "One limit bid, two market bids, no fees, slippage not exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(99_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(95_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask price 1.0, one market bid, limit order smaller size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(2)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 50_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(50_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(99_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(47_500_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask price 2.0, one market bid, limit order smaller size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(99_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(95_000_000),
        },
    },
    BTreeMap::new();
    "One limit bid price 1.0, one market ask, limit order smaller size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(2)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(99_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(200_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(190_000_000),
        },
    },
    BTreeMap::new();
    "One limit bid price 2.0, one market ask, limit order smaller size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    true,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(95_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(95_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask price 1.0, one market bid, limit order smaller size, 1% maker fee, 5% taker fee, no slippage, limit order placed in same block as market order"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(150_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    true,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(95_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(95_000_000),
        },
    },
    BTreeMap::new();
    "One limit bid price 1.0, one market ask, limit order smaller size, 1% maker fee, 5% taker fee, no slippage, limit order placed in same block as market order"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    max_slippage: Udec128::ZERO,
                },
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(99_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(95_000_000),
        },
    },
    BTreeMap::new();
    "One limit bid price 1.0, one market asks from same address, limit order same size, 1% maker fee, 5% taker fee, no slippage, limit order placed in same block as market order"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    max_slippage: Udec128::ZERO,
                },
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(50_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(99_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(95_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask price 1.0, two market bids from same address, limit order same size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        ),
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(99_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(95_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        2 => btree_map! {
            usdc::DENOM.clone() => BalanceChange::Unchanged,
            dango::DENOM.clone() => BalanceChange::Unchanged,
        },
    },
    BTreeMap::new();
    "One limit ask price 1.0, one market bid matched two market bids from other address unmatched are refunded correctly, limit order same size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 200_000_000,
            },
        ),
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(99_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(95_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone() => BalanceChange::Unchanged,
        },
    },
    BTreeMap::new();
    "One limit bid price 1.0, one market ask matched two market asks from other address unmatched are refunded correctly, limit order same size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(110_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                usdc::DENOM.clone() => 110_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                dango::DENOM.clone() => 200_000_000,
            },
        ),
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(108_900_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(110_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(95_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(9_500_000),
        },
    },
    BTreeMap::new();
    "One limit bid price 1.0, one market ask matched two market asks from other address one partially matched one unmatched are refunded correctly, limit order same size, 1% maker fee, 5% taker fee, no slippage"
)]
#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(110_000_000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                },
            ],
            coins! {
                dango::DENOM.clone() => 110_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
                CreateMarketOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    max_slippage: Udec128::ZERO,
                },
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        ),
    ],
    false,
    Udec128::new_percent(1),
    Udec128::new_percent(5),
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(110_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(108_900_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(95_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(100_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9_500_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(10_000_000),
        },
    },
    BTreeMap::new();
    "One limit ask price 1.0, one market bid matched two market bids from other address one partially matched one unmatched are refunded correctly, limit order same size, 1% maker fee, 5% taker fee, no slippage"
)]
fn market_order_clearing(
    limit_orders_and_funds: Vec<(Vec<CreateLimitOrderRequest>, Coins)>,
    market_orders_and_funds: Vec<(Vec<CreateMarketOrderRequest>, Coins)>,
    limits_and_markets_in_same_block: bool,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    expected_balance_changes: BTreeMap<usize, BTreeMap<Denom, BalanceChange>>,
    expected_limit_orders_after: BTreeMap<OrderId, (Direction, Udec128, Uint128, Uint128, usize)>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set maker and taker fee rates to 0 for simplicity
    let mut app_config: AppConfig = suite.query_app_config().unwrap();
    app_config.maker_fee_rate = Bounded::new(maker_fee_rate).unwrap();
    app_config.taker_fee_rate = Bounded::new(taker_fee_rate).unwrap();

    // Update the app config
    suite
        .configure(
            &mut accounts.owner, // Must be the chain owner
            None,                // No chain config update
            Some(app_config),    // App config update
        )
        .should_succeed();

    // Register oracle price source for USDC and DANGO. Needed for volume tracking in cron_execute
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Record balances for users
    suite.balances().record_many(accounts.users());

    // Build create limit order transactions
    let num_limit_order_users = limit_orders_and_funds.len();
    let mut submit_limit_order_txs = vec![];
    for (user, (limit_orders, limit_order_funds)) in
        accounts.users_mut().zip(limit_orders_and_funds)
    {
        let msg = Message::execute(
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: limit_orders,
                cancels: None,
            },
            limit_order_funds,
        )
        .unwrap();
        let tx = user
            .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
            .unwrap();
        submit_limit_order_txs.push(tx);
    }

    // Build create market order transactions
    let mut create_market_order_txs = vec![];
    for (user, (market_orders, market_order_funds)) in accounts
        .users_mut()
        .skip(num_limit_order_users)
        .zip(market_orders_and_funds)
    {
        let msg = Message::execute(
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: market_orders,
                creates_limit: vec![],
                cancels: None,
            },
            market_order_funds,
        )
        .unwrap();

        let tx = user
            .sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
            .unwrap();
        create_market_order_txs.push(tx);
    }

    // Execute the transactions in a block
    if limits_and_markets_in_same_block {
        suite
            .make_block(
                submit_limit_order_txs
                    .into_iter()
                    .chain(create_market_order_txs)
                    .collect(),
            )
            .block_outcome
            .tx_outcomes
            .into_iter()
            .for_each(|outcome| {
                outcome.should_succeed();
            });
    } else {
        suite
            .make_block(submit_limit_order_txs)
            .block_outcome
            .tx_outcomes
            .into_iter()
            .for_each(|outcome| {
                outcome.should_succeed();
            });
        suite
            .make_block(create_market_order_txs)
            .block_outcome
            .tx_outcomes
            .into_iter()
            .for_each(|outcome| {
                outcome.should_succeed();
            });
    }

    // Assert that the balance changes are as expected
    let users = accounts.users().collect::<Vec<_>>();
    for (index, expected_change) in expected_balance_changes {
        let user = users[index];
        suite.balances().should_change(user, expected_change);
    }

    // Assert that the limit orders are as expected
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and(|orders| {
            // println!("orders: {:?}", orders);
            assert_eq!(orders.len(), expected_limit_orders_after.len());
            expected_limit_orders_after.iter().all(
                |(order_id, (direction, price, amount, remaining, user_index))| {
                    let queried_order = orders.get(order_id).unwrap();
                    queried_order.direction == *direction
                        && queried_order.price == price.convert_precision().unwrap()
                        && queried_order.amount == *amount
                        && queried_order.remaining == remaining.checked_into_dec().unwrap()
                        && queried_order.user == users[*user_index].address()
                },
            )
        });
}

#[test_case(
    CreateLimitOrderRequest {
        base_denom: eth::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Ask,
        amount: NonZero::new_unchecked(Uint128::new(9307)),
        price: NonZero::new_unchecked(Udec128_24::new(1000000)),
    },
    coins! {
        eth::DENOM.clone() => 9307,
    },
    CreateMarketOrderRequest {
        base_denom: eth::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(500000)),
        max_slippage: Udec128::new_percent(8),
    },
    coins! {
        usdc::DENOM.clone() => 500000,
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(9307),
        usdc::DENOM.clone() => BalanceChange::Increased(498750),
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Unchanged,
        usdc::DENOM.clone() => BalanceChange::Decreased(500000),
    }
    ; "limit ask matched with market bid market size limiting factor market order gets refunded"
)]
#[test_case(
    CreateLimitOrderRequest {
        base_denom: eth::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(500000)),
        price: NonZero::new_unchecked(Udec128_24::new_bps(1)),
    },
    coins! {
        usdc::DENOM.clone() => 50,
    },
    CreateMarketOrderRequest {
        base_denom: eth::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Ask,
        amount: NonZero::new_unchecked(Uint128::new(9999)),
        max_slippage: Udec128::new_percent(8),
    },
    coins! {
        eth::DENOM.clone() => 9999,
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(9974),
        usdc::DENOM.clone() => BalanceChange::Decreased(50),
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(9999),
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    }
    ; "limit bid matched with market ask market size limiting factor market order gets refunded"
)]
#[test_case(
    CreateLimitOrderRequest {
        base_denom: eth::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(9999)),
        price: NonZero::new_unchecked(Udec128_24::new_bps(1)),
    },
    coins! {
        usdc::DENOM.clone() => 1,
    },
    CreateMarketOrderRequest {
        base_denom: eth::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Ask,
        amount: NonZero::new_unchecked(Uint128::new(500000)),
        max_slippage: Udec128::new_percent(8),
    },
    coins! {
        eth::DENOM.clone() => 500000,
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(9974),
        usdc::DENOM.clone() => BalanceChange::Decreased(1),
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(9999),
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    }
    ; "limit bid matched with market ask limit size too small market order is consumed with zero output"
)]
fn market_order_matched_results_in_zero_output(
    limit_order: CreateLimitOrderRequest,
    limit_funds: Coins,
    market_order: CreateMarketOrderRequest,
    market_funds: Coins,
    expected_balance_changes_limit_order_user: BTreeMap<Denom, BalanceChange>,
    expected_balance_changes_market_order_user: BTreeMap<Denom, BalanceChange>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Register oracle price source for USDC and ETH. Needed for volume tracking in cron_execute
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
                eth::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    suite.balances().record_many(accounts.users());

    // Create a limit order with a small amount and price
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![limit_order],
                cancels: None,
            },
            limit_funds,
        )
        .should_succeed();

    // Create a market order with a small amount and price,
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![market_order],
                creates_limit: vec![],
                cancels: None,
            },
            market_funds,
        )
        .should_succeed();

    // No matching could take place so both users should have unchanged balances
    // since before the market order was created.
    suite
        .balances()
        .should_change(&accounts.user1, expected_balance_changes_limit_order_user);
    suite
        .balances()
        .should_change(&accounts.user2, expected_balance_changes_market_order_user);
}

#[test]
fn cron_execute_gracefully_handles_oracle_price_failure() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    suite.balances().record_many(accounts.users());

    // Submit a limit order
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(1000000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => 1000000,
            },
        )
        .should_succeed();

    // Submit another limit from user2
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(1000000)),
                    price: NonZero::new_unchecked(Udec128_24::new(1)),
                }],
                cancels: None,
            },
            coins! {
                usdc::DENOM.clone() => 1000000,
            },
        )
        .should_succeed();

    // ------ Assert that the orders were matched and filled correctly ------

    // There should be no orders in the order book
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(BTreeMap::new());

    // Balances should have been updated
    suite.balances().should_change(&accounts.user1, btree_map! {
        dango::DENOM.clone() => BalanceChange::Decreased(1000000),
        usdc::DENOM.clone() => BalanceChange::Increased(997500),
    });
    suite.balances().should_change(&accounts.user2, btree_map! {
        dango::DENOM.clone() => BalanceChange::Increased(996000),
        usdc::DENOM.clone() => BalanceChange::Decreased(1000000),
    });
}

#[test]
fn market_orders_are_sorted_by_price_ascending() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set maker and taker fee rates to 0 for simplicity
    // TODO: make this configurable in `TestOptions`
    let mut app_config: AppConfig = suite.query_app_config().unwrap();
    app_config.maker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    app_config.taker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    suite
        .configure(
            &mut accounts.owner, // Must be the chain owner
            None,                // No chain config update
            Some(app_config),    // App config update
        )
        .should_succeed();

    // Register oracle price source for USDC and DANGO. Needed for volume tracking in cron_execute
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                usdc::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // No matter which user places the orders they outcome should be the same.
    for i in 0..2 {
        let (first_user, second_user) = if i == 0 {
            (&mut accounts.user2, &mut accounts.user3)
        } else {
            (&mut accounts.user3, &mut accounts.user2)
        };

        suite
            .balances()
            .record_many([&*first_user, &*second_user, &accounts.user1]);

        // Place limit ASK with user 1. This is the first order in the order book. Since
        // no matching orders exist this order will be the only order in the resting order book
        // at the end of the block.
        suite
            .execute(
                &mut accounts.user1,
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(1000000)),
                        price: NonZero::new_unchecked(Udec128_24::new(1)),
                    }],
                    cancels: None,
                },
                coins! {
                    dango::DENOM.clone() => 1000000,
                },
            )
            .should_succeed();

        // Submit two market orders from different users in the same block. First user
        // places a market order with 0% slippage and second user places a market order
        // with 5% slippage.
        let txs = vec![
            first_user
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates_market: vec![CreateMarketOrderRequest {
                                    base_denom: dango::DENOM.clone(),
                                    quote_denom: usdc::DENOM.clone(),
                                    direction: Direction::Bid,
                                    amount: NonZero::new_unchecked(Uint128::new(1000000)),
                                    max_slippage: Udec128::ZERO,
                                }],
                                creates_limit: vec![],
                                cancels: None,
                            },
                            coins! {
                                usdc::DENOM.clone() => 1000000,
                            },
                        )
                        .unwrap(),
                    ]),
                    &suite.chain_id,
                    100_000,
                )
                .unwrap(),
            second_user
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates_market: vec![CreateMarketOrderRequest {
                                    base_denom: dango::DENOM.clone(),
                                    quote_denom: usdc::DENOM.clone(),
                                    direction: Direction::Bid,
                                    amount: NonZero::new_unchecked(Uint128::new(1050000)),
                                    max_slippage: Udec128::new_percent(5),
                                }],
                                creates_limit: vec![],
                                cancels: None,
                            },
                            coins! {
                                usdc::DENOM.clone() => 1102500, // amount * (best_ask_price * (1 + max_slippage)) = 1050000 * (1 * (1 + 0.05))
                            },
                        )
                        .unwrap(),
                    ]),
                    &suite.chain_id,
                    100_000,
                )
                .unwrap(),
        ];

        // Make a block with the order submissions. Ensure all transactions were
        // successful.
        suite
            .make_block(txs)
            .block_outcome
            .tx_outcomes
            .into_iter()
            .for_each(|outcome| {
                outcome.should_succeed();
            });

        // Assert that the second_user gets fully matched and the first_user gets no match
        // since the second_user's order is at a higher price and fully consumes the limit order.
        suite.balances().should_change(&accounts.user1, btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(1000000),
            usdc::DENOM.clone() => BalanceChange::Increased(1000000),
        });
        suite.balances().should_change(first_user, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Unchanged, // no match and deposit is refunded
            dango::DENOM.clone() => BalanceChange::Unchanged, // no match
        });
        suite.balances().should_change(second_user, btree_map! {
            usdc::DENOM.clone() => BalanceChange::Decreased(1000000), // Cleared at resting book price of 1.0 consumes 1000000 USDC and refunds 50000 USDC
            dango::DENOM.clone() => BalanceChange::Increased(1000000),
        });
    }
}

/// During the `match_orders` function call, there may be an order that's popped
/// out of the iterator but didn't find a match. Considering the following case:
/// - id 1, limit ask, price 100, amount 1
/// - id 2, limit bid, price 101, amount 1
/// - id 3, market bid, price 100, amount 1
/// Since order 2 has the better price, it will be matched against 1.
/// Market order 3 will be popped out of the iterator, but not finding a match.
/// In this case, we need to handle the cancelation and refund of this order.
#[test]
fn refund_left_over_market_bid() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set maker and taker fee rates to 0 for simplicity
    // TODO: make this configurable in `TestOptions.`
    let mut app_config: AppConfig = suite.query_app_config().unwrap();
    app_config.maker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    app_config.taker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    suite
        .configure(
            &mut accounts.owner, // Must be the chain owner
            None,                // No chain config update
            Some(app_config),    // App config update
        )
        .should_succeed();

    // Block 1: we make it such that a mid price of 100 is recorded.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(2)),
                        price: NonZero::new_unchecked(Udec128_24::new(100)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(1)),
                        price: NonZero::new_unchecked(Udec128_24::new(100)),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => 2,
                usdc::DENOM.clone() => 100,
            },
        )
        .should_succeed();

    // Query the mid price to make sure it's accurate.
    suite
        .query_wasm_smart(contracts.dex, QueryRestingOrderBookStateRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(RestingOrderBookState {
            best_bid_price: None,
            best_ask_price: Some(Udec128_24::new(100)),
            mid_price: Some(Udec128_24::new(100)),
        });

    suite
        .balances()
        .record_many([&accounts.user1, &accounts.user2, &accounts.user3]);

    // Block 2: submit two orders:
    // - user 2 submits the limit order that will be matched;
    // - user 3 submits the market order that will be left over.
    // Make sure the limit order is submitted first, meaning it's more
    suite
        .make_block(vec![
            accounts
                .user2
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates_market: vec![],
                                creates_limit: vec![CreateLimitOrderRequest {
                                    base_denom: dango::DENOM.clone(),
                                    quote_denom: usdc::DENOM.clone(),
                                    direction: Direction::Bid,
                                    amount: NonZero::new_unchecked(Uint128::new(1)),
                                    price: NonZero::new_unchecked(Udec128_24::new(101)),
                                }],
                                cancels: None,
                            },
                            coins! {
                                usdc::DENOM.clone() => 101,
                            },
                        )
                        .unwrap(),
                    ]),
                    &suite.chain_id,
                    100_000,
                )
                .unwrap(),
            accounts
                .user3
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates_market: vec![CreateMarketOrderRequest {
                                    base_denom: dango::DENOM.clone(),
                                    quote_denom: usdc::DENOM.clone(),
                                    direction: Direction::Bid,
                                    amount: NonZero::new_unchecked(Uint128::new(1)),
                                    max_slippage: Udec128::ZERO,
                                }],
                                creates_limit: vec![],
                                cancels: None,
                            },
                            coins! {
                                usdc::DENOM.clone() => 101,
                            },
                        )
                        .unwrap(),
                    ]),
                    &suite.chain_id,
                    100_000,
                )
                .unwrap(),
        ])
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    // Check user 1 and user 2 balances.
    // The order should match with range 100-101. Since previous block's mid
    // price was 100, which is within the range, so the orders settle at 100.
    suite.balances().should_change(&accounts.user1, btree_map! {
        dango::DENOM.clone() => BalanceChange::Unchanged,
        usdc::DENOM.clone() => BalanceChange::Increased(100),
    });
    suite.balances().should_change(&accounts.user2, btree_map! {
        dango::DENOM.clone() => BalanceChange::Increased(1),
        usdc::DENOM.clone() => BalanceChange::Decreased(100),
    });

    // THE IMPORTANT PART: make sure user 3 has received the refund; or in other
    // words, his balance should be unchanged.
    suite.balances().should_change(&accounts.user3, btree_map! {
        dango::DENOM.clone() => BalanceChange::Unchanged,
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    });
}

/// This is the same as the previous test (`refund_left_over_market_bid`), but
/// on the different side of the book.
///
/// The setup:
/// - mid price: 100
/// - resting order book: limit bid, price 100, amount 1
/// - limit ask, price 99, amount 1
/// - market ask, price 100, amount 1
#[test]
fn refund_left_over_market_ask() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set maker and taker fee rates to 0 for simplicity
    // TODO: make this configurable in `TestOptions.`
    let mut app_config: AppConfig = suite.query_app_config().unwrap();
    app_config.maker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    app_config.taker_fee_rate = Bounded::new(Udec128::ZERO).unwrap();
    suite
        .configure(
            &mut accounts.owner, // Must be the chain owner
            None,                // No chain config update
            Some(app_config),    // App config update
        )
        .should_succeed();

    // Block 1: we make it such that a mid price of 100 is recorded.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(2)),
                        price: NonZero::new_unchecked(Udec128_24::new(100)),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(1)),
                        price: NonZero::new_unchecked(Udec128_24::new(100)),
                    },
                ],
                cancels: None,
            },
            coins! {
                dango::DENOM.clone() => 1,
                usdc::DENOM.clone() => 200,
            },
        )
        .should_succeed();

    // Query the mid price to make sure it's accurate.
    suite
        .query_wasm_smart(contracts.dex, QueryRestingOrderBookStateRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(RestingOrderBookState {
            best_bid_price: Some(Udec128_24::new(100)),
            best_ask_price: None,
            mid_price: Some(Udec128_24::new(100)),
        });

    suite
        .balances()
        .record_many([&accounts.user1, &accounts.user2, &accounts.user3]);

    // Block 2: submit two orders:
    // - user 2 submits the limit order that will be matched;
    // - user 3 submits the market order that will be left over.
    // Make sure the limit order is submitted first, meaning it's more
    suite
        .make_block(vec![
            accounts
                .user2
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates_market: vec![],
                                creates_limit: vec![CreateLimitOrderRequest {
                                    base_denom: dango::DENOM.clone(),
                                    quote_denom: usdc::DENOM.clone(),
                                    direction: Direction::Ask,
                                    amount: NonZero::new_unchecked(Uint128::new(1)),
                                    price: NonZero::new_unchecked(Udec128_24::new(99)),
                                }],
                                cancels: None,
                            },
                            coins! {
                                dango::DENOM.clone() => 1,
                            },
                        )
                        .unwrap(),
                    ]),
                    &suite.chain_id,
                    100_000,
                )
                .unwrap(),
            accounts
                .user3
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates_market: vec![CreateMarketOrderRequest {
                                    base_denom: dango::DENOM.clone(),
                                    quote_denom: usdc::DENOM.clone(),
                                    direction: Direction::Ask,
                                    amount: NonZero::new_unchecked(Uint128::new(1)),
                                    max_slippage: Udec128::ZERO,
                                }],
                                creates_limit: vec![],
                                cancels: None,
                            },
                            coins! {
                                dango::DENOM.clone() => 1,
                            },
                        )
                        .unwrap(),
                    ]),
                    &suite.chain_id,
                    100_000,
                )
                .unwrap(),
        ])
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    // Check user 1 and user 2 balances.
    // The order should match with range 99-100. Since previous block's mid
    // price was 100, which is within the range, so the orders settle at 100.
    suite.balances().should_change(&accounts.user1, btree_map! {
        dango::DENOM.clone() => BalanceChange::Increased(1),
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    });
    suite.balances().should_change(&accounts.user2, btree_map! {
        dango::DENOM.clone() => BalanceChange::Decreased(1),
        usdc::DENOM.clone() => BalanceChange::Increased(100),
    });

    // THE IMPORTANT PART: make sure user 3 has received the refund; or in other
    // words, his balance should be unchanged.
    suite.balances().should_change(&accounts.user3, btree_map! {
        dango::DENOM.clone() => BalanceChange::Unchanged,
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    });
}
