use {
    dango_testing::{BridgeOp, TestOption, constants::mock_ethereum, setup_test_naive},
    dango_types::{
        constants::{atom, dango, eth, usdc},
        dex::{
            self, CancelOrderRequest, CreateOrderRequest, Direction, OrderId, OrderResponse,
            PairParams, PairUpdate, Price, QueryOrdersByPairRequest, QueryOrdersRequest,
        },
        gateway::Remote,
    },
    grug::{
        Addr, Addressable, BalanceChange, Bounded, Coins, Denom, Message, MultiplyFraction,
        NonEmpty, NonZero, NumberConst, QuerierExt, ResultExt, Signer, StdError, StdResult,
        Udec128, Udec128_6, Uint128, btree_map, coins,
    },
    std::collections::{BTreeMap, BTreeSet},
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
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::ZERO), // incorrect!
                )],
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
                creates: vec![CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::ZERO), // incorrect!
                )],
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
                creates: vec![CreateOrderRequest::new_limit(
                    atom::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100)),
                )],
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

#[test_case(
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    None,
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!1) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Price::new(1),
            amount: Uint128::new(100),
            remaining: Udec128_6::new(100),
        },
    };
    "one submission no cancellations"
)]
#[test_case(
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "one submission cancels one order"
)]
#[test_case(
    vec![
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
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
            price: Price::new(1),
            amount: Uint128::new(100),
            remaining: Udec128_6::new(100),
        },
    };
    "two submission cancels one order"
)]
#[test_case(
    vec![
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1), OrderId::new(!2)]))),
    coins! { usdc::DENOM.clone() => 200 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "two submission cancels both orders"
)]
#[test_case(
    vec![
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
    ],
    Some(CancelOrderRequest::All),
    coins! { usdc::DENOM.clone() => 200 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "two submission cancel all"
)]
#[test_case(
    vec![
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
        CreateOrderRequest::new_limit(
            dango::DENOM.clone(),
            usdc::DENOM.clone(),
            Direction::Bid,
            NonZero::new_unchecked(Price::new(1)),
            NonZero::new_unchecked(Uint128::new(100)),
        ),
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    coins! { usdc::DENOM.clone() => 199 },
    btree_map! {},
    btree_map! {}
    => panics "insufficient funds for batch updating orders";
    "two submission insufficient funds"
)]
fn submit_and_cancel_orders(
    creates: Vec<CreateOrderRequest>,
    cancels: Option<CancelOrderRequest>,
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
                creates,
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
                creates: vec![],
                cancels,
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
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Price::new(1),
            amount: Uint128::new(100),
            remaining: Udec128_6::new(100),
        },
    };
    "submit one order then cancel it and submit it again"
)]
#[test_case(
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(50)),
    )],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Increased(50) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Price::new(1),
            amount: Uint128::new(50),
            remaining: Udec128_6::new(50),
        },
    };
    "submit one order then cancel it and place a new order using half of the funds"
)]
#[test_case(
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(200)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Price::new(1),
            amount: Uint128::new(200),
            remaining: Udec128_6::new(200),
        },
    };
    "submit one order then cancel it and place a new order using more funds"
)]
#[test_case(
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(200)),
    )],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Price::new(1),
            amount: Uint128::new(200),
            remaining: Udec128_6::new(200),
        },
    }
    => panics "insufficient funds";
    "submit one order then cancel it and place a new order with insufficient funds"
)]
#[test_case(
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(100)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([OrderId::new(!1)]))),
    vec![CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new(1)),
        NonZero::new_unchecked(Uint128::new(150)),
    )],
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(50) },
    btree_map! {
        OrderId::new(!2) => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            price: Price::new(1),
            amount: Uint128::new(150),
            remaining: Udec128_6::new(150),
        },
    };
    "submit one order then cancel it and place a new order excess funds are returned"
)]
fn submit_orders_then_cancel_and_submit_in_same_message(
    initial_orders: Vec<CreateOrderRequest>,
    initial_funds: Coins,
    cancellations: Option<CancelOrderRequest>,
    new_orders: Vec<CreateOrderRequest>,
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
                creates: initial_orders,
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
                creates: new_orders,
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
            creates: vec![CreateOrderRequest::new_limit(
                dango::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Bid,
                NonZero::new_unchecked(Price::new(1)),
                NonZero::new_unchecked(Uint128::new(100)),
            )],
            cancels: None,
        },
        coins! { usdc::DENOM.clone() => 100 },
    )
    .unwrap();

    let cancel_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates: vec![],
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
        usdc::DENOM.clone() => BalanceChange::Unchanged
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
                        domain: mock_ethereum::DOMAIN,
                        contract: mock_ethereum::USDC_WARP,
                    },
                    amount: Uint128::new(100_000_000_000),
                    recipient: accounts.user1.address(),
                },
                BridgeOp {
                    remote: Remote::Warp {
                        domain: mock_ethereum::DOMAIN,
                        contract: mock_ethereum::ETH_WARP,
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
            let price = Price::new(price);
            let amount_base = Uint128::new(amount);

            let (amount, funds) = match direction {
                Direction::Bid => {
                    let amount_quote = amount_base.checked_mul_dec_ceil(price).unwrap();
                    let funds = Coins::one(quote_denom.clone(), amount_quote).unwrap();
                    (amount_quote, funds)
                },
                Direction::Ask => {
                    let funds = Coins::one(base_denom.clone(), amount_base).unwrap();
                    (amount_base, funds)
                },
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![CreateOrderRequest::new_limit(
                        base_denom,
                        quote_denom,
                        direction,
                        NonZero::new_unchecked(price),
                        NonZero::new_unchecked(amount),
                    )],
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

#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(100)), // ceil(199 * 0.5)
    ),
    coins! { usdc::DENOM.clone() => 100 },
    Uint128::new(100),
    Uint128::new(100),
    None;
    "bid equal to minimum order size"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(99)), // ceil(198 * 0.5)
    ),
    coins! { usdc::DENOM.clone() => 100 },
    Uint128::new(100),
    Uint128::new(100),
    Some("order size (99 bridge/usdc) is less than the minimum (100 bridge/usdc)");
    "bid smaller than minimum order size"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(200)),
    ),
    coins! { dango::DENOM.clone() => 200 },
    Uint128::new(100),
    Uint128::new(100),
    None;
    "ask equal to minimum order size"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(198)),
    ),
    coins! { dango::DENOM.clone() => 198 },
    Uint128::new(100),
    Uint128::new(100),
    Some("order size (99 bridge/usdc) is less than the minimum (100 bridge/usdc)");
    "ask smaller than minimum order size"
)]
fn limit_order_minimum_order_size(
    order: CreateOrderRequest,
    funds: Coins,
    min_order_size_quote: Uint128,
    min_order_size_base: Uint128,
    expected_error: Option<&str>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Update the pair params with the minimum order size

    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            bucket_sizes: BTreeSet::new(),
                            swap_fee_rate: pair_params.swap_fee_rate,
                            pool_type: pair_params.pool_type.clone(),
                            min_order_size_quote,
                            min_order_size_base,
                        },
                    }])),
                    Coins::new(),
                )
                .should_succeed();
            true
        });

    // Submit the order
    match expected_error {
        None => {
            suite
                .execute(
                    &mut accounts.user1,
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates: vec![order],
                        cancels: None,
                    },
                    funds,
                )
                .should_succeed();
        },

        Some(error) => {
            suite
                .execute(
                    &mut accounts.user1,
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates: vec![order],
                        cancels: None,
                    },
                    funds,
                )
                .should_fail_with_error(error);
        },
    }
}

#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(200)),
    ),
    CreateOrderRequest::new_market(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        Bounded::new_unchecked(Udec128::ZERO),
        NonZero::new_unchecked(Uint128::new(100)),
    ),
    coins! { dango::DENOM.clone() => 200 },
    coins! { usdc::DENOM.clone() => 100 },
    Uint128::new(100),
    Uint128::new(100),
    None;
    "bid equal to minimum order size no slippage"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(200)),
    ),
    CreateOrderRequest::new_market(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        Bounded::new_unchecked(Udec128::ZERO),
        NonZero::new_unchecked(Uint128::new(99)),
    ),
    coins! { dango::DENOM.clone() => 200 },
    // User only needs to deposit 99 uusdc to create this order, which is
    // smaller than the minimum, hence should be rejected.
    // Even if user send 100 (more than necessary, and satisfies the minimum)
    // the contract needs to still properly reject the order.
    coins! { usdc::DENOM.clone() => 100 },
    Uint128::new(100),
    Uint128::new(100),
    Some("order size (99 bridge/usdc) is less than the minimum (100 bridge/usdc)");
    "bid smaller than minimum order size no slippage"
)]
#[test_case(
    // This test case is equal to the above test case
    // 'bid smaller than minimum order size no slippage'
    // but with slippage accounted order size is large
    // enough and so it should succeed, whereas the above
    // test case should fail. It is important that these
    // two tests remain equal except for the slippage
    // and expected error.
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(200)),
    ),
    CreateOrderRequest::new_market(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        Bounded::new_unchecked(Udec128::new_percent(1)),
        NonZero::new_unchecked(Uint128::new(100)), // ceil(198 * (0.5 * (1 + 0.01)))
    ),
    coins! { dango::DENOM.clone() => 200 },
    coins! { usdc::DENOM.clone() => 100 },
    Uint128::new(100),
    Uint128::new(100),
    None;
    "bid smaller than minimum order size but larger with slippage accounted for"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(100)), // 200 * 0.5
    ),
    CreateOrderRequest::new_market(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        Bounded::new_unchecked(Udec128::ZERO),
        NonZero::new_unchecked(Uint128::new(200)),
    ),
    coins! { usdc::DENOM.clone() => 100 },
    coins! { dango::DENOM.clone() => 200 },
    Uint128::new(100),
    Uint128::new(100),
    None;
    "ask equal to minimum order size no slippage"
)]
#[test_case(
    // This test case is equal to the above test case
    // 'ask equal to minimum order size no slippage'
    // but with slippage accounted order size is smaller
    // enough and so it should fail, whereas the above
    // test case should succeed. It is important that these
    // two tests remain equal except for the slippage
    // and expected error.
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(100)), // 200 * 0.5
    ),
    CreateOrderRequest::new_market(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        Bounded::new_unchecked(Udec128::new_percent(1)), // this indicates a price of 0.5 * (1 - 0.01) = 0.495
        NonZero::new_unchecked(Uint128::new(200)),       // this indicates a base amount of 200 * 0.495 = 99, smaller than minimum order size (100)
    ),
    coins! { usdc::DENOM.clone() => 100 },
    coins! { dango::DENOM.clone() => 200 },
    Uint128::new(100),
    Uint128::new(100),
    Some("order size (99 bridge/usdc) is less than the minimum (100 bridge/usdc)");
    "ask equal to minimum order size but smaller with slippage accounted for"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_percent(50)),
        NonZero::new_unchecked(Uint128::new(100)), // 200 * 0.5
    ),
    CreateOrderRequest::new_market(
        dango::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        Bounded::new_unchecked(Udec128::ZERO),
        NonZero::new_unchecked(Uint128::new(198)),
    ),
    coins! { usdc::DENOM.clone() => 100 },
    coins! { dango::DENOM.clone() => 198 },
    Uint128::new(100),
    Uint128::new(100),
    Some("order size (99 bridge/usdc) is less than the minimum (100 bridge/usdc)");
    "ask smaller than minimum order size no slippage"
)]
fn market_order_minimum_order_size(
    limit_order: CreateOrderRequest,
    market_order: CreateOrderRequest,
    limit_order_funds: Coins,
    market_order_funds: Coins,
    min_order_size_quote: Uint128,
    min_order_size_base: Uint128,
    expected_error: Option<&str>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Update the pair params with the minimum order size
    suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            min_order_size_quote,
                            min_order_size_base,
                            ..pair_params.clone()
                        },
                    }])),
                    Coins::new(),
                )
                .should_succeed();
            true
        });

    // Submit the limit order to create a resting order book
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![limit_order],
                cancels: None,
            },
            limit_order_funds,
        )
        .should_succeed();

    // Submit the order
    match expected_error {
        None => {
            suite
                .execute(
                    &mut accounts.user1,
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates: vec![market_order],
                        cancels: None,
                    },
                    market_order_funds,
                )
                .should_succeed();
        },
        Some(error) => {
            suite
                .execute(
                    &mut accounts.user1,
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdateOrders {
                        creates: vec![market_order],
                        cancels: None,
                    },
                    market_order_funds,
                )
                .should_fail_with_error(error);
        },
    }
}

#[test]
fn orders_cannot_be_created_for_non_existing_pair() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Submit limit order
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    eth::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100)),
                )],
                cancels: None,
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "pair not found with base `{}` and quote `{}`",
            dango::DENOM.clone(),
            eth::DENOM.clone()
        ));

    // Submit the market order
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    eth::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100)),
                )],
                cancels: None,
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "pair not found with base `{}` and quote `{}`",
            dango::DENOM.clone(),
            eth::DENOM.clone()
        ));
}

/// Ensure that if user creates an order and then immediately cancels it, the
/// user should get back the original deposit amount. If handled incorrectly,
/// the user may get back less due to rounding.
///
/// In this test case, the user creates a limit BUY order at price 100 with
/// quote asset amount 150. The order's size will be: floor(150 / 100) = 1.
/// The DEX contract should understand that this should only require a deposit
/// of 1 * 100 = 100 quote asset deposit, and refund the excess 50, at the time
/// of order creation. Then, if the user cancels the order, he gets the remaining
/// 100 back.
#[test]
fn create_and_cancel_order_with_remainder() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    suite.balances().record(&accounts.user1);

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest::new_limit(
                    eth::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(100)),
                    NonZero::new_unchecked(Uint128::new(150)),
                )],
                cancels: None,
            },
            Coins::one(usdc::DENOM.clone(), 150).unwrap(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![],
                cancels: Some(CancelOrderRequest::All),
            },
            Coins::default(),
        )
        .should_succeed();

    suite.balances().should_change(&accounts.user1, btree_map! {
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    });
}
