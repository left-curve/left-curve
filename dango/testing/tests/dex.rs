use {
    dango_testing::setup_test_naive,
    dango_types::{
        account::single::Params,
        account_factory::AccountParams,
        config::AppConfig,
        constants::{ATOM_DENOM, BTC_DENOM, DANGO_DENOM, ETH_DENOM, USDC_DENOM, XRP_DENOM},
        dex::{
            self, CreateLimitOrderRequest, CurveInvariant, Direction, OrderId, OrderIds,
            OrderResponse, PairId, PairParams, PairUpdate, QueryOrdersByPairRequest,
            QueryOrdersRequest, QueryReserveRequest,
        },
        oracle::{self, PriceSource},
    },
    grug::{
        Addr, Addressable, BalanceChange, Bounded, Coin, CoinPair, Coins, Denom, Fraction, Inner,
        IsZero, MaxLength, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, QuerierExt,
        ResultExt, Signer, StdResult, Udec128, Uint128, UniqueVec, btree_map, coins,
    },
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
    },
    test_case::test_case,
};

#[test]
fn cannot_submit_orders_in_non_existing_pairs() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateLimitOrderRequest {
                    base_denom: ATOM_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction: Direction::Bid,
                    amount: Uint128::new(100),
                    price: Udec128::new(1),
                }],
                cancels: None,
            },
            Coins::one(USDC_DENOM.clone(), 1).unwrap(),
        )
        .should_fail_with_error(format!(
            "pair not found with base `{}` and quote `{}`",
            ATOM_DENOM.clone(),
            USDC_DENOM.clone()
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
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(199), // Receives one less due to fee
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(199), // Receives one less due to fee
        },
        6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
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
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
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
        !2 =>  5,
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(4),   // half filled, receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(188), // -200 deposit, +12 refund
        },
        !3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(174),
        },
        6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
        },
        !7 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(4),
            USDC_DENOM.clone()  => BalanceChange::Decreased(88), // -150 deposit, +62 refund
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
        !2 => 10,
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(19), // Receives one less due to fee
            USDC_DENOM.clone()  => BalanceChange::Decreased(450), // -600 deposit, +150 refund
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(224), // Receives one less due to fee
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(224),
        },
        6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
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
        !2 => 10,
        !3 => 10,
         6 =>  5,
    },
    btree_map! {
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(24),
            USDC_DENOM.clone()  => BalanceChange::Decreased(688), // -750 deposit, +62 refund
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(273), // Receives two less due to fee
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(273), // Receives two less due to fee
        },
        6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(136), // refund: floor(5 * 27.5) = 137 minus 1 due to fee
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
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Register oracle price source for USDC and DANGO. Needed for volume tracking in cron_execute
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                USDC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
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
                DANGO_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
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
            let order_id = (order_id + 1) as OrderId;
            match direction {
                Direction::Bid => (!order_id, signer.address()),
                Direction::Ask => (order_id, signer.address()),
            }
        })
        .collect::<BTreeMap<_, _>>();

    // Track the users' balances.
    suite
        .balances()
        .record_many(users_by_order_id.values().copied());

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .zip(accounts.users_mut())
        .map(|((direction, price, amount), signer)| {
            let price = Udec128::new(price);
            let amount = Uint128::new(amount);

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(USDC_DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(DANGO_DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction,
                        amount,
                        price,
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
            .should_change(users_by_order_id[&order_id], changes);
    }

    // Check the remaining unfilled orders.
    let orders = suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .unwrap()
        .into_iter()
        .map(|(order_id, order)| (order_id, order.remaining.into_inner()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(orders, remaining_orders);
}

#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    None,
    coins! { USDC_DENOM.clone() => 100 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !1 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(100),
            remaining: Uint128::new(100),
        },
    };
    "one submission no cancellations"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    coins! { USDC_DENOM.clone() => 100 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "one submission cancels one order"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
    ],
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    coins! { USDC_DENOM.clone() => 200 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(100),
            remaining: Uint128::new(100),
        },
    };
    "two submission cancels one order"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
    ],
    Some(OrderIds::Some(BTreeSet::from([!1, !2]))),
    coins! { USDC_DENOM.clone() => 200 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "two submission cancels both orders"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
    ],
    Some(OrderIds::All),
    coins! { USDC_DENOM.clone() => 200 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {};
    "two submission cancel all"
)]
#[test_case(
    vec![
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
    ],
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    coins! { USDC_DENOM.clone() => 199 },
    btree_map! {},
    btree_map! {}
    => panics "insufficient funds for batch updating orders";
    "two submission insufficient funds"
)]
fn submit_and_cancel_orders(
    submissions: Vec<CreateLimitOrderRequest>,
    cancellations: Option<OrderIds>,
    funds: Coins,
    expected_balance_changes: BTreeMap<Denom, BalanceChange>,
    expected_orders_after: BTreeMap<OrderId, OrderResponse>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Record the user's balance.
    suite.balances().record(accounts.user1.address());

    // Add order to the order book.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: submissions,
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
                cancels: cancellations,
            },
            coins! { DANGO_DENOM.clone() => 1 },
        )
        .should_succeed();

    // Check that the user balance has not changed.
    suite
        .balances()
        .should_change(accounts.user1.address(), expected_balance_changes);

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
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    Coins::new(),
    btree_map! { USDC_DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(100),
            remaining: Uint128::new(100),
        },
    };
    "submit one order then cancel it and submit it again"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(50),
        price: Udec128::new(1),
    }],
    Coins::new(),
    btree_map! { USDC_DENOM.clone() => BalanceChange::Increased(50) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(50),
            remaining: Uint128::new(50),
        },
    };
    "submit one order then cancel it and place a new order using half of the funds"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(200),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(200),
            remaining: Uint128::new(200),
        },
    };
    "submit one order then cancel it and place a new order using more funds"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(200),
        price: Udec128::new(1),
    }],
    Coins::new(),
    btree_map! { USDC_DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(200),
            remaining: Uint128::new(200),
        },
    }
    => panics "insufficient funds";
    "submit one order then cancel it and place a new order with insufficient funds"
)]
#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(100),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    Some(OrderIds::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
        direction: Direction::Bid,
        amount: Uint128::new(150),
        price: Udec128::new(1),
    }],
    coins! { USDC_DENOM.clone() => 100 },
    btree_map! { USDC_DENOM.clone() => BalanceChange::Decreased(50) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            price: Udec128::new(1),
            amount: Uint128::new(150),
            remaining: Uint128::new(150),
        },
    };
    "submit one order then cancel it and place a new order excess funds are returned"
)]
fn submit_orders_then_cancel_and_submit_in_same_message(
    initial_orders: Vec<CreateLimitOrderRequest>,
    initial_funds: Coins,
    cancellations: Option<OrderIds>,
    new_orders: Vec<CreateLimitOrderRequest>,
    second_funds: Coins,
    expected_balance_changes: BTreeMap<Denom, BalanceChange>,
    expected_orders_after: BTreeMap<OrderId, OrderResponse>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

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
    suite.balances().record(accounts.user1.address());

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
        .should_change(accounts.user1.address(), expected_balance_changes);

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
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Record the user's balance
    suite.balances().record(accounts.user1.address());

    // Build and sign a transaction with two messages: submit an order and cancel the order
    let submit_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates: vec![CreateLimitOrderRequest {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::new(100),
                price: Udec128::new(1),
            }],
            cancels: None,
        },
        coins! { USDC_DENOM.clone() => 100 },
    )
    .unwrap();

    let cancel_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::BatchUpdateOrders {
            creates: vec![],
            cancels: Some(dex::OrderIds::Some(BTreeSet::from([!1]))),
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
    suite
        .balances()
        .should_change(accounts.user1.address(), btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged
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
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (DANGO_DENOM.clone(), USDC_DENOM.clone()),
    None,
    None,
    btree_map! {
        !1 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        !2 => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        3 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
        4 => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
    };
    "dango/usdc no pagination"
)]
#[test_case(
    vec![
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (ETH_DENOM.clone(), USDC_DENOM.clone()),
    None,
    None,
    btree_map! {
        !5 => (Direction::Bid, Udec128::new(20), Uint128::new(10)),
        6 => (Direction::Ask, Udec128::new(25), Uint128::new(10)),
    };
    "eth/usdc no pagination"
)]
#[test_case(
    vec![
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (DANGO_DENOM.clone(), USDC_DENOM.clone()),
    None,
    Some(3),
    btree_map! {
        !1 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        !2 => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        3 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
    };
    "dango/usdc with limit no start after"
)]
#[test_case(
    vec![
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (DANGO_DENOM.clone(), USDC_DENOM.clone()),
    Some(3),
    None,
    btree_map! {
        4 => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
    };
    "dango/usdc with start after"
)]
#[test_case(
    vec![
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 30, 10), // !0
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 10, 10), // !1
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 40, 10), //  2
        ((DANGO_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 50, 10), //  3
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Bid, 20, 10), // !4
        ((ETH_DENOM.clone(), USDC_DENOM.clone()), Direction::Ask, 25, 10), //  5
    ],
    (DANGO_DENOM.clone(), USDC_DENOM.clone()),
    Some(!2),
    Some(2),
    btree_map! {
        !1 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        3 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
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
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .map(|((base_denom, quote_denom), direction, price, amount)| {
            let price = Udec128::new(price);
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
                    creates: vec![CreateLimitOrderRequest {
                        base_denom,
                        quote_denom,
                        direction,
                        amount,
                        price,
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
                        && queried_order.price == *price
                        && queried_order.amount == *amount
                        && queried_order.remaining == *amount
                        && queried_order.user == accounts.user1.address()
                })
        });
}

#[test]
fn only_owner_can_create_passive_pool() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/pool/xrp/usdc").unwrap();

    // Attempt to create pair as non-owner. Should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: XRP_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: lp_denom.clone(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::ZERO),
                    order_depth: 100,
                    order_spacing: Udec128::new_bps(1),
                },
            }]),
            Coins::new(),
        )
        .should_fail_with_error("only the owner can update a trading pair parameters");

    // Attempt to create pair as owner. Should succeed.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: XRP_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: lp_denom.clone(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::ZERO),
                    order_depth: 100,
                    order_spacing: Udec128::new_bps(1),
                },
            }]),
            Coins::new(),
        )
        .should_succeed();
}

#[test_case(
    coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 100,
    },
    Uint128::new(100);
    "provision at pool ratio"
)]
#[test_case(
    coins! {
        DANGO_DENOM.clone() => 50,
        USDC_DENOM.clone() => 50,
    },
    Uint128::new(50);
    "provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 50,
    },
    Uint128::new(73);
    "provision at different ratio"
)]
fn provide_liquidity(provision: Coins, expected_lp_balance: Uint128) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/pool/dango/usdc").unwrap();

    // Owner first provides some initial liquidity.
    let initial_reserves = coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone()  => 100,
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
            },
            initial_reserves.clone(),
        )
        .should_succeed();

    // Record the users initial balances.
    suite
        .balances()
        .record_many(accounts.users().map(|user| user.address()));

    // Execute all the provisions.
    let mut expected_pool_balances = initial_reserves.clone();

    // record the dex balance
    suite.balances().record(contracts.dex.address());

    // Execute provide liquidity
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
            },
            provision.clone(),
        )
        .should_succeed();

    // Ensure that the dex balance has increased by the expected amount.
    suite.balances().should_change(
        contracts.dex.address(),
        balance_changes_from_coins(provision.clone(), Coins::new()),
    );

    // Ensure user's balance has decreased by the expected amount and that
    // LP tokens have been minted.
    suite.balances().should_change(
        accounts.user1.address(),
        balance_changes_from_coins(
            coins! { lp_denom.clone() => expected_lp_balance },
            provision.clone(),
        ),
    );

    // Check that the reserves in pool object were updated correctly.
    suite
        .query_wasm_smart(contracts.dex, dex::QueryReserveRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        })
        .should_succeed_and_equal(
            expected_pool_balances
                .insert_many(provision)
                .unwrap()
                .take_pair((DANGO_DENOM.clone(), USDC_DENOM.clone()))
                .unwrap(),
        );
}

#[test_case(
    Uint128::new(100),
    coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone()  => 100,
    };
    "withdrawa all"
)]
#[test_case(
    Uint128::new(50),
    coins! {
        DANGO_DENOM.clone() => 50,
        USDC_DENOM.clone()  => 50,
    };
    "withdraw half"
)]
fn withdraw_liquidity(lp_burn_amount: Uint128, expected_funds_returned: Coins) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/pool/dango/usdc").unwrap();

    // Owner first provides some initial liquidity.
    let initial_reserves = coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone()  => 100,
    };

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
            },
            initial_reserves.clone(),
        )
        .should_succeed();

    // User provides some liquidity.
    let provided_funds = coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 100,
    };

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
            },
            provided_funds.clone(),
        )
        .should_succeed();

    // record user and dex balances
    suite
        .balances()
        .record_many(vec![accounts.user1.address(), contracts.dex.address()]);

    // withdraw liquidity
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::WithdrawLiquidity {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
            },
            coins! { lp_denom.clone() => lp_burn_amount },
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    suite.balances().should_change(
        accounts.user1.address(),
        balance_changes_from_coins(
            expected_funds_returned.clone(),
            coins! { lp_denom.clone() => lp_burn_amount },
        ),
    );

    // Assert that the dex balance has decreased by the expected amount.
    suite.balances().should_change(
        contracts.dex.address(),
        balance_changes_from_coins(Coins::new(), expected_funds_returned.clone()),
    );

    // Assert pool reserves are updated correctly
    suite
        .query_wasm_smart(contracts.dex, dango_types::dex::QueryReserveRequest {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        })
        .should_succeed_and_equal({
            initial_reserves
                .clone()
                .insert_many(provided_funds)
                .unwrap()
                .deduct_many(expected_funds_returned)
                .unwrap()
                .take_pair((DANGO_DENOM.clone(), USDC_DENOM.clone()))
                .unwrap()
        });
}

#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    BTreeMap::new(),
    None,
    coins! {
        USDC_DENOM.clone() => 500000,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    };
    "1:1 pool no swap fee one step route input 100% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    coins! {
        DANGO_DENOM.clone() => 500000,
    },
    BTreeMap::new(),
    None,
    coins! {
        USDC_DENOM.clone() => 333333,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1500000,
            USDC_DENOM.clone() => 666667,
        },
    };
    "1:1 pool no swap fee one step route input 50% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    coins! {
        DANGO_DENOM.clone() => 333333,
    },
    BTreeMap::new(),
    None,
    coins! {
        USDC_DENOM.clone() => 249999,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1333333,
            USDC_DENOM.clone() => 1000000 - 249999,
        },
    };
    "1:1 pool no swap fee one step route input 33% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
        (BTC_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            BTC_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![
        PairId {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        },
        PairId {
            base_denom: BTC_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        }
    ],
    coins! {
        DANGO_DENOM.clone() => 500000,
    },
    BTreeMap::new(),
    None,
    coins! {
        BTC_DENOM.clone() => 249999,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000 + 500000,
            USDC_DENOM.clone() => 1000000 - 333333,
        },
        (BTC_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            BTC_DENOM.clone() => 1000000 - 249999,
            USDC_DENOM.clone() => 1000000 + 333333,
        },
    };
    "1:1 pools no swap fee input 100% of pool liquidity two step route"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    BTreeMap::new(),
    Some(500000u128.into()),
    coins! {
        USDC_DENOM.clone() => 500000,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    };
    "1:1 pool no swap fee one step route input 100% of pool liquidity output is not less than minimum output"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    BTreeMap::new(),
    Some(499999u128.into()),
    coins! {
        USDC_DENOM.clone() => 500000,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    };
    "1:1 pool no swap fee one step route input 100% of pool liquidity output is less than minimum output"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => Udec128::new_bps(1),
    },
    None,
    coins! {
        USDC_DENOM.clone() => 499950,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 1000000 - 499950,
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
    expected_pool_reserves_after: BTreeMap<(Denom, Denom), Coins>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    for ((base_denom, quote_denom), swap_fee_rate) in swap_fee_rates {
        if swap_fee_rate.is_zero() {
            continue;
        }

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
                        &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                            params: PairParams {
                                lp_denom: pair_params.lp_denom.clone(),
                                swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                                curve_invariant: pair_params.curve_invariant.clone(),
                                order_depth: pair_params.order_depth,
                                order_spacing: pair_params.order_spacing,
                            },
                        }]),
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
        .record_many(vec![accounts.user1.address(), contracts.dex.address()]);

    // User swaps
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountIn {
                route: MaxLength::new_unchecked(UniqueVec::try_from(route).unwrap()),
                minimum_output,
            },
            swap_funds.clone(),
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    suite.balances().should_change(
        accounts.user1.address(),
        balance_changes_from_coins(expected_out.clone(), swap_funds.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    suite.balances().should_change(
        contracts.dex.address(),
        balance_changes_from_coins(swap_funds.clone(), expected_out.clone()),
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

#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 500000).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 1000000).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    };
    "1:1 pool no swap fee one step route output 50% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 333333).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 499999,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 499999).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000 + 499999,
            USDC_DENOM.clone() => 1000000 - 333333,
        },
    };
    "1:1 pool no swap fee one step route output 33% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 250000).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 333333,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 333333).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000 + 333333,
            USDC_DENOM.clone() => 1000000 - 250000,
        },
    };
    "1:1 pool no swap fee one step route output 25% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 1000000).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 1000000).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    }
    => panics "insufficient liquidity" ;
    "1:1 pool no swap fee one step route output 100% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 500000).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 999999,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 1000000).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    }
    => panics "insufficient input for swap" ;
    "1:1 pool no swap fee one step route output 50% of pool liquidity insufficient funds"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 500000).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 1100000,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 1000000).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 500000,
        },
    };
    "1:1 pool no swap fee one step route output 50% of pool liquidity excessive funds returned"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
        (BTC_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            BTC_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![
        PairId {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        },
        PairId {
            base_denom: BTC_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        },
    ],
    Coin::new(BTC_DENOM.clone(), 250000).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    BTreeMap::new(),
    Coin::new(DANGO_DENOM.clone(), 499999).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000 + 499999,
            USDC_DENOM.clone() => 1000000 - 333333,
        },
        (BTC_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            BTC_DENOM.clone() => 1000000 - 250000,
            USDC_DENOM.clone() => 1000000 + 333333,
        },
    };
    "1:1 pool no swap fee two step route output 25% of pool liquidity"
)]
#[test_case(
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 1000000,
            USDC_DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: DANGO_DENOM.clone(),
        quote_denom: USDC_DENOM.clone(),
    }],
    Coin::new(USDC_DENOM.clone(), 499950).unwrap(),
    coins! {
        DANGO_DENOM.clone() => 1000000,
    },
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => Udec128::new_bps(1),
    },
    Coin::new(DANGO_DENOM.clone(), 1000000).unwrap(),
    btree_map! {
        (DANGO_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            DANGO_DENOM.clone() => 2000000,
            USDC_DENOM.clone() => 1000000 - 499950,
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
    expected_pool_reserves_after: BTreeMap<(Denom, Denom), Coins>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Update the pairs with the new swap fee rates.
    for ((base_denom, quote_denom), swap_fee_rate) in swap_fee_rates {
        if swap_fee_rate.is_zero() {
            continue;
        }

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
                    &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: base_denom.clone(),
                        quote_denom: quote_denom.clone(),
                        params,
                    }]),
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
        .record_many(vec![accounts.user1.address(), contracts.dex.address()]);

    // User swaps
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountOut {
                route: MaxLength::new_unchecked(UniqueVec::try_from(route).unwrap()),
                output: NonZero::new(exact_out.clone()).unwrap(),
            },
            swap_funds.clone(),
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    let expected_out_coins: Coins = vec![exact_out].try_into().unwrap();
    let expected_in_coins: Coins = vec![expected_in].try_into().unwrap();
    suite.balances().should_change(
        accounts.user1.address(),
        balance_changes_from_coins(expected_out_coins.clone(), expected_in_coins.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    suite.balances().should_change(
        contracts.dex.address(),
        balance_changes_from_coins(expected_in_coins.clone(), expected_out_coins.clone()),
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

#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::new_percent(1),
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::from(49751),
                price: Udec128::new_percent(20100),
            },
        ],
    ],
    vec![
        coins! {
            USDC_DENOM.clone() => 49751 * 201,
        },
    ],
    btree_map! {
        !1u64 => (Udec128::new_percent(20100), Uint128::from(49751), Direction::Bid),
    },
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000,
            USDC_DENOM.clone() => 200 * 10000000,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Unchanged,
        USDC_DENOM.clone() => BalanceChange::Increased(49751 * 201),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone() => BalanceChange::Decreased(49751 * 201),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 one percent fee no matching orders")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::ZERO,
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::from(49751),
                price: Udec128::new_percent(20100),
            },
        ],
    ],
    vec![
        coins! {
            USDC_DENOM.clone() => 49751 * 201,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 - 49751,
            USDC_DENOM.clone() => 200 * 10000000 + 49751 * 201,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Decreased(49751),
        USDC_DENOM.clone() => BalanceChange::Increased(49751 * 201),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Increased(49751),
            USDC_DENOM.clone() => BalanceChange::Decreased(49751 * 201),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 no fee user bid order exactly matches passive order")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::new_percent(1),
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::from(47783),
                price: Udec128::new_percent(20300),
            },
        ],
    ],
    vec![
        coins! {
            USDC_DENOM.clone() => 47783 * 203,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 - 47783,
            USDC_DENOM.clone() => 200 * 10000000 + 47783 * 203,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Decreased(47783),
        USDC_DENOM.clone() => BalanceChange::Increased(47783 * 203),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Increased(47783),
            USDC_DENOM.clone() => BalanceChange::Decreased(47783 * 203),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 one percent fee user bid order partially fills passive order")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::new_percent(1),
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::from(157784),
                price: Udec128::new_percent(20300),
            },
        ],
    ],
    vec![
        coins! {
            USDC_DENOM.clone() => 157784 * 203,
        },
    ],
    btree_map! {
        !1u64 => (Udec128::new_percent(20300), Uint128::from(10000), Direction::Bid),
    },
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 - 147784,
            USDC_DENOM.clone() => 200 * 10000000 + 147784 * 203,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Decreased(147784),
        USDC_DENOM.clone() => BalanceChange::Increased(157784 * 203),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Increased(147784),
            USDC_DENOM.clone() => BalanceChange::Decreased(157784 * 203),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 one percent fee user bid order fully fills passive order with amount remaining after")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::ZERO,
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Ask,
                amount: Uint128::from(50251),
                price: Udec128::new_percent(19900),
            },
        ],
    ],
    vec![
        coins! {
            ETH_DENOM.clone() => 50251,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 + 50251,
            USDC_DENOM.clone() => 200 * 10000000 - 50251 * 199,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Increased(50251),
        USDC_DENOM.clone() => BalanceChange::Decreased(50251 * 199),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Decreased(50251),
            USDC_DENOM.clone() => BalanceChange::Increased(50251 * 199),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 no fee user ask order exactly matches passive order")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::ZERO,
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Ask,
                amount: Uint128::from(30000),
                price: Udec128::new_percent(19900),
            },
        ],
    ],
    vec![
        coins! {
            ETH_DENOM.clone() => 30000,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 + 30000,
            USDC_DENOM.clone() => 200 * 10000000 - 30000 * 199,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Increased(30000),
        USDC_DENOM.clone() => BalanceChange::Decreased(30000 * 199),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Decreased(30000),
            USDC_DENOM.clone() => BalanceChange::Increased(30000 * 199),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 no fee user ask order partially fills passive order")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::ZERO,
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Ask,
                amount: Uint128::from(60251),
                price: Udec128::new_percent(19900),
            },
        ],
    ],
    vec![
        coins! {
            ETH_DENOM.clone() => 60251,
        },
    ],
    btree_map! {
        1u64 => (Udec128::new_percent(19900), Uint128::from(10000), Direction::Ask),
    },
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 + 50251,
            USDC_DENOM.clone() => 200 * 10000000 - 50251 * 199,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Increased(60251),
        USDC_DENOM.clone() => BalanceChange::Decreased(50251 * 199),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Decreased(60251),
            USDC_DENOM.clone() => BalanceChange::Increased(50251 * 199),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 no fee user ask order fully fills passive order with amount remaining after")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    10,
    Udec128::new_percent(1),
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Ask,
                amount: Uint128::from(162284),
                price: Udec128::new_percent(19700),
            },
        ],
        vec![
            CreateLimitOrderRequest {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::from(157784),
                price: Udec128::new_percent(20300),
            },
        ],
    ],
    vec![
        coins! {
            ETH_DENOM.clone() => 162284,
        },
        coins! {
            USDC_DENOM.clone() => 157784 * 203,
        },
    ],
    BTreeMap::new(),
    btree_map! {
        (ETH_DENOM.clone(), USDC_DENOM.clone()) => coins! {
            ETH_DENOM.clone() => 10000000 + 4500, // only the remaining amount of the ask order traded against the passive pool
            USDC_DENOM.clone() => 200 * 10000000 - 4500 * 197,
        }.try_into().unwrap(),
    },
    btree_map! {
        ETH_DENOM.clone() => BalanceChange::Increased(162284 - 157784),
        USDC_DENOM.clone() => BalanceChange::Decreased(4500 * 197),
    },
    vec![
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Decreased(162284),
            USDC_DENOM.clone() => BalanceChange::Increased(162284 * 197),
        },
        btree_map! {
            ETH_DENOM.clone() => BalanceChange::Increased(157784),
            USDC_DENOM.clone() => BalanceChange::Decreased(157784 * 197),
        }
    ]
    ; "xyk pool balance 1:200 tick size 1 one percent fee three users with multiple orders")]
fn curve_on_orderbook(
    curve_invariant: CurveInvariant,
    order_spacing: Udec128,
    order_depth: u64,
    swap_fee_rate: Udec128,
    pool_liquidity: Coins,
    orders: Vec<Vec<CreateLimitOrderRequest>>,
    order_creation_funds: Vec<Coins>,
    expected_orders_after_clearing: BTreeMap<OrderId, (Udec128, Uint128, Direction)>,
    expected_reserves_after_clearing: BTreeMap<(Denom, Denom), CoinPair>,
    expected_dex_balance_changes: BTreeMap<Denom, BalanceChange>,
    expected_user_balance_changes: Vec<BTreeMap<Denom, BalanceChange>>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

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
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
        })
        .should_succeed_and(|pair_params: &PairParams| {
            // Provide liquidity with owner account
            suite
                .execute(
                    &mut accounts.owner,
                    contracts.dex,
                    &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: ETH_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            curve_invariant,
                            swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                            order_depth,
                            order_spacing,
                        },
                    }]),
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
                USDC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
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
                ETH_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::new_percent(2000),
                    precision: 6,
                    timestamp: 1730802926,
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
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
            },
            pool_liquidity.clone(),
        )
        .should_succeed();

    // Record dex and user balances
    suite.balances().record(contracts.dex.address());
    suite
        .balances()
        .record_many(accounts.users().map(|user| user.address()));

    // Create txs for all the orders from all users.
    let txs = accounts
        .users_mut()
        .zip(orders)
        .zip(order_creation_funds)
        .map(|((user, orders), order_creation_funds)| {
            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: orders,
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

    // Assert that dex balances have changed as expected
    suite
        .balances()
        .should_change(contracts.dex.address(), expected_dex_balance_changes);

    // Assert that user balances have changed as expected
    for (user, expected_user_balance_change) in accounts.users().zip(expected_user_balance_changes)
    {
        suite
            .balances()
            .should_change(user.address(), expected_user_balance_change);
    }

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
                assert_eq!(order.price, price);
                assert_eq!(order.remaining, remaining);
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
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Register oracle price source for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                USDC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
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
                DANGO_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
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
            Coins::one(USDC_DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap();

    let mut user2_addr_1 = accounts.user2;
    let mut user2_addr_2 = user2_addr_1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            AccountParams::Spot(Params::new(user2_addr_1.username.clone())),
            Coins::one(DANGO_DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap();

    // Query volumes before, should be 0
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    // Submit a new order with user1 address 1
    suite
        .execute(
            &mut user1_addr_1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateLimitOrderRequest {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction: Direction::Bid,
                    amount: Uint128::new(100_000_000),
                    price: Udec128::new(1),
                }],
                cancels: None,
            },
            Coins::one(USDC_DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // User2 submit an opposite matching order with address 1
    suite
        .execute(
            &mut user2_addr_1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateLimitOrderRequest {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction: Direction::Ask,
                    amount: Uint128::new(100_000_000),
                    price: Udec128::new(1),
                }],
                cancels: None,
            },
            Coins::one(DANGO_DENOM.clone(), 100_000_000).unwrap(),
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
        .should_succeed_and_equal(Uint128::new(100));

    // Query the volume for username user2, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user2_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    // Query the volume for user1 address 1, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    // Query the volume for user2 address 1, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    // Query the volume for user1 address 2, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    // Query the volume for user2 address 2, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::ZERO);

    // Submit a new order with user1 address 2
    suite
        .execute(
            &mut user1_addr_2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateLimitOrderRequest {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction: Direction::Bid,
                    amount: Uint128::new(100_000_000),
                    price: Udec128::new(1),
                }],
                cancels: None,
            },
            Coins::one(USDC_DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Submit a new opposite matching order with user2 address 2
    suite
        .execute(
            &mut user2_addr_2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateLimitOrderRequest {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction: Direction::Ask,
                    amount: Uint128::new(100_000_000),
                    price: Udec128::new(1),
                }],
                cancels: None,
            },
            Coins::one(DANGO_DENOM.clone(), 100_000_000).unwrap(),
        )
        .should_succeed();

    // Query the volume for username user1, should be 200
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(200));

    // Query the volume for username user2, should be 200
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user2_addr_1.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(200));

    // Query the volume for all addresses, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(100));

    // Query the volume for both usernames since timestamp after first trade, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user1_addr_1.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: user2_addr_1.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::new(100));

    // Query the volume for both users address 1 since timestamp after first trade, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_1.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::ZERO);

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_1.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::ZERO);

    // Query the volume for both users address 2 since timestamp after first trade, should be 100
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user1_addr_2.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::new(100));

    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: user2_addr_2.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::new(100));
}

#[test]
fn volume_tracking_works_with_multiple_orders_from_same_user() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Register oracle price source for USDC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                USDC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
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
                DANGO_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: 1730802926,
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Register oracle price source for BTC
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                BTC_DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::from_str("85248.71").unwrap(),
                    precision: 8,
                    timestamp: 1730802926,
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Submit two orders for DANGO/USDC and one for BTC/USDC with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(100_000_000),
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(100_000_000),
                        price: Udec128::from_str("1.01").unwrap(),
                    },
                    CreateLimitOrderRequest {
                        base_denom: BTC_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(117304),
                        price: Udec128::from_str("852.485845").unwrap(),
                    },
                ],
                cancels: None,
            },
            Coins::one(USDC_DENOM.clone(), 301_000_000).unwrap(),
        )
        .should_succeed();

    // Submit matching orders with user2
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Ask,
                        amount: Uint128::new(200_000_000),
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: BTC_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Ask,
                        amount: Uint128::new(117304),
                        price: Udec128::from_str("852.485845").unwrap(),
                    },
                ],
                cancels: None,
            },
            coins! {
                DANGO_DENOM.clone() => 200_000_000,
                BTC_DENOM.clone() => 117304,
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
        .should_succeed_and_equal(Uint128::new(300));

    // Query the volume for username user2, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(300));

    // Query the volume for user1 address, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(300));

    // Query the volume for user2 address, should be 300
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(300));

    // Query the volume for both usernames since timestamp after first trade, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user1.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::ZERO);
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::ZERO);

    // Submit new orders with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(100_000_000),
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(100_000_000),
                        price: Udec128::from_str("1.01").unwrap(),
                    },
                    CreateLimitOrderRequest {
                        base_denom: BTC_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(117304),
                        price: Udec128::from_str("852.485845").unwrap(),
                    },
                    CreateLimitOrderRequest {
                        base_denom: BTC_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Bid,
                        amount: Uint128::new(117304),
                        price: Udec128::from_str("937.7344336").unwrap(),
                    },
                ],
                cancels: None,
            },
            coins! {
                USDC_DENOM.clone() => 411_000_000,
            },
        )
        .should_succeed();

    // Submit matching orders with user2
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateLimitOrderRequest {
                        base_denom: DANGO_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Ask,
                        amount: Uint128::new(300_000_000),
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: BTC_DENOM.clone(),
                        quote_denom: USDC_DENOM.clone(),
                        direction: Direction::Ask,
                        amount: Uint128::new(117304 * 2),
                        price: Udec128::from_str("85248.71")
                            .unwrap()
                            .checked_inv()
                            .unwrap(),
                    },
                ],
                cancels: None,
            },
            coins! {
                DANGO_DENOM.clone() => 300_000_000,
                BTC_DENOM.clone() => 117304 * 2,
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
        .should_succeed_and_equal(Uint128::new(700));

    // Query the volume for username user2, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(700));

    // Query the volume for user1 address, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user1.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(700));

    // Query the volume for user2 address, should be 700
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user2.address(),
            since: None,
        })
        .should_succeed_and_equal(Uint128::new(700));

    // Query the volume for both usernames since timestamp after second trade, should be zero
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user1.username.clone(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Uint128::ZERO);
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeByUserRequest {
            user: accounts.user2.username.clone(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Uint128::ZERO);

    // Query the volume for both addresses since timestamp after the first trade, should be 400
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user1.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::new(400));
    suite
        .query_wasm_smart(contracts.dex, dex::QueryVolumeRequest {
            user: accounts.user2.address(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Uint128::new(400));
}
