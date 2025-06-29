use {
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
        },
        gateway::{Remote, WarpRemote},
        oracle::{self, PriceSource},
    },
    grug::{
        Addr, Addressable, BalanceChange, Bounded, Coin, CoinPair, Coins, Denom, Fraction, Inner,
        MaxLength, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, QuerierExt,
        ResultExt, Signer, StdError, StdResult, Timestamp, Udec128, Uint128, UniqueVec, btree_map,
        coin_pair, coins,
    },
    hyperlane_types::constants::ethereum,
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !3 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(199), // Receives one less due to fee
        },
        5 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(199), // Receives one less due to fee
        },
        6 => btree_map! {
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
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !3 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        5 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        6 => btree_map! {
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
        !2 =>  5,
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(9), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(4),   // half filled, receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(188), // -200 deposit, +12 refund
        },
        !3 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174), // Receives one less due to fee
        },
        5 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(174),
        },
        6 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Unchanged,
        },
        !7 => btree_map! {
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
        !2 => 10,
        !3 => 10,
         6 => 10,
    },
    btree_map! {
        !1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(19), // Receives one less due to fee
            usdc::DENOM.clone()  => BalanceChange::Decreased(450), // -600 deposit, +150 refund
        },
        !2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !3 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(224), // Receives one less due to fee
        },
        5 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(224),
        },
        6 => btree_map! {
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
        !2 => 10,
        !3 => 10,
         6 =>  5,
    },
    btree_map! {
        !1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(24),
            usdc::DENOM.clone()  => BalanceChange::Decreased(688), // -750 deposit, +62 refund
        },
        !2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !3 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Unchanged,
            usdc::DENOM.clone()  => BalanceChange::Decreased(100),
        },
        4 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(273), // Receives two less due to fee
        },
        5 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(10),
            usdc::DENOM.clone()  => BalanceChange::Increased(273), // Receives two less due to fee
        },
        6 => btree_map! {
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
            let order_id = (order_id + 1) as OrderId;
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
            let price = Udec128::new(price);
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
        .map(|(order_id, order)| (order_id, order.remaining.into_inner()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(orders, remaining_orders);
}

#[test_case(
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    None,
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !1 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
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
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: Udec128::new(1),
        },
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
    coins! { usdc::DENOM.clone() => 200 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: Udec128::new(1),
        },
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([!1, !2]))),
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
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: Udec128::new(1),
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
            price: Udec128::new(1),
        },
        CreateLimitOrderRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            direction: Direction::Bid,
            amount: NonZero::new_unchecked(Uint128::new(100)),
            price: Udec128::new(1),
        },
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
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
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Unchanged },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(50)),
        price: Udec128::new(1),
    }],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Increased(50) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(200)),
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(200)),
        price: Udec128::new(1),
    }],
    Coins::new(),
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(100) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(100)),
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    Some(CancelOrderRequest::Some(BTreeSet::from([!1]))),
    vec![CreateLimitOrderRequest {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
        direction: Direction::Bid,
        amount: NonZero::new_unchecked(Uint128::new(150)),
        price: Udec128::new(1),
    }],
    coins! { usdc::DENOM.clone() => 100 },
    btree_map! { usdc::DENOM.clone() => BalanceChange::Decreased(50) },
    btree_map! {
        !2 => OrderResponse {
            user: Addr::mock(1), // Just a placeholder. User1 address is used in assertion.
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
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
                price: Udec128::new(1),
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
            cancels: Some(dex::CancelOrderRequest::Some(BTreeSet::from([!1]))),
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
        !1 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        !2 => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        3 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
        4 => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
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
        !5 => (Direction::Bid, Udec128::new(20), Uint128::new(10)),
        6 => (Direction::Ask, Udec128::new(25), Uint128::new(10)),
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
        !1 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        !2 => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        3 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
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
    Some(3),
    None,
    btree_map! {
        4 => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
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
    // For this test, we need some ETH and USDC for user1.
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption {
        bridge_ops: |accounts| {
            vec![
                BridgeOp {
                    remote: Remote::Warp(WarpRemote {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::USDC_WARP,
                    }),
                    amount: Uint128::new(100_000_000_000),
                    recipient: accounts.user1.address(),
                },
                BridgeOp {
                    remote: Remote::Warp(WarpRemote {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::WETH_WARP,
                    }),
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
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom,
                        quote_denom,
                        direction,
                        amount: NonZero::new_unchecked(amount),
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
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let lp_denom = Denom::try_from("dex/pool/xrp/usdc").unwrap();

    // Attempt to create pair as non-owner. Should fail.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: xrp::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params: PairParams {
                    lp_denom: lp_denom.clone(),
                    pool_type: PassiveLiquidity::Xyk {
                        order_spacing: Udec128::new_bps(1),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
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
                base_denom: xrp::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params: PairParams {
                    lp_denom: lp_denom.clone(),
                    pool_type: PassiveLiquidity::Xyk {
                        order_spacing: Udec128::new_bps(1),
                    },
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                },
            }]),
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
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(100);
    "provision at pool ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(50);
    "provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
    },
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(72);
    "provision at different ratio"
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
                    &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee),
                            pool_type,
                        },
                    }]),
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

#[test_case(
    Uint128::new(99),
    Udec128::new_permille(5),
    coins! {
        dango::DENOM.clone() => 99,
        usdc::DENOM.clone()  => 99,
    };
    "withdrawa all"
)]
#[test_case(
    Uint128::new(50),
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
                    &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee),
                            pool_type: pair_params.pool_type.clone(),
                        },
                    }]),
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
        usdc::DENOM.clone() => 497500,
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
        usdc::DENOM.clone() => 331666,
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
        usdc::DENOM.clone() => 247814,
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
        eth::DENOM.clone() => 247814,
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
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 2000000,
            usdc::DENOM.clone() => 500000,
        },
    } => panics "output amount is below the minimum: 497500 < 500000" ;
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
    Some(497500u128.into()),
    coins! {
        usdc::DENOM.clone() => 497500,
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
        usdc::DENOM.clone() => 499950,
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
                        &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                            base_denom: base_denom.clone(),
                            quote_denom: quote_denom.clone(),
                            params: PairParams {
                                lp_denom: pair_params.lp_denom.clone(),
                                swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                                pool_type: pair_params.pool_type.clone(),
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
        .record_many([&accounts.user1.address(), &contracts.dex]);

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
        &accounts.user1,
        balance_changes_from_coins(expected_out.clone(), swap_funds.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    suite.balances().should_change(
        &contracts.dex,
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
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000,
            usdc::DENOM.clone() => 1000000,
        },
    },
    vec![PairId {
        base_denom: dango::DENOM.clone(),
        quote_denom: usdc::DENOM.clone(),
    }],
    Coin::new(usdc::DENOM.clone(), 500000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1002006,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1002006).unwrap(),
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
    Coin::new(usdc::DENOM.clone(), 333333).unwrap(),
    coins! {
        dango::DENOM.clone() => 500751,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 500751).unwrap(),
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
    Coin::new(usdc::DENOM.clone(), 250000).unwrap(),
    coins! {
        dango::DENOM.clone() => 333779,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 333779).unwrap(),
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
    Coin::new(usdc::DENOM.clone(), 1000000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1000000).unwrap(),
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
    Coin::new(usdc::DENOM.clone(), 500000).unwrap(),
    coins! {
        dango::DENOM.clone() => 999999,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1000000).unwrap(),
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
    Coin::new(usdc::DENOM.clone(), 500000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1100000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1002006).unwrap(),
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
    Coin::new(eth::DENOM.clone(), 250000).unwrap(),
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
        (eth::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 501758).unwrap(),
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
    Coin::new(usdc::DENOM.clone(), 499950).unwrap(),
    coins! {
        dango::DENOM.clone() => 1000000,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_bps(1),
    },
    Coin::new(dango::DENOM.clone(), 1000000).unwrap(),
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
        .record_many([&accounts.user1.address(), &contracts.dex]);

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
        &accounts.user1,
        balance_changes_from_coins(expected_out_coins.clone(), expected_in_coins.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    suite.balances().should_change(
        &contracts.dex,
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
    PassiveLiquidity::Xyk {
        order_spacing: Udec128::ONE,
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
                price: Udec128::new_percent(20100),
            },
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 49751 * 201,
        },
    ],
    btree_map! {
        !1u64 => (Udec128::new_percent(20100), Uint128::from(49751), Direction::Bid),
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
                price: Udec128::new_percent(20100),
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
                price: Udec128::new_percent(20200),
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
                price: Udec128::new_percent(20300),
            },
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 157784 * 203,
        },
    ],
    btree_map! {
        !1u64 => (Udec128::new_percent(20300), Uint128::from(10000), Direction::Bid),
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
                price: Udec128::new_percent(19900),
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
                price: Udec128::new_percent(19900),
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
                price: Udec128::new_percent(19900),
            },
        ],
    ],
    vec![
        coins! {
            eth::DENOM.clone() => 60251,
        },
    ],
    btree_map! {
        1u64 => (Udec128::new_percent(19900), Uint128::from(10000), Direction::Ask),
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
                price: Udec128::new_percent(19800),
            },
        ],
        vec![
            CreateLimitOrderRequest {
                base_denom: eth::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                direction: Direction::Bid,
                amount: NonZero::new_unchecked(Uint128::from(157784)),
                price: Udec128::new_percent(20200),
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
    expected_orders_after_clearing: BTreeMap<OrderId, (Udec128, Uint128, Direction)>,
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
                    &dex::ExecuteMsg::BatchUpdatePairs(vec![PairUpdate {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: pair_params.lp_denom.clone(),
                            pool_type,
                            swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
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
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                creates_market: vec![],
                creates_limit: vec![CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Bid,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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

    // Register oracle price source for BTC
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

    // Submit two orders for DANGO/USDC and one for BTC/USDC with user1
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
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: Udec128::from_str("1.01").unwrap(),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: Udec128::from_str("852.485845").unwrap(),
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
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: Udec128::from_str("852.485845").unwrap(),
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
                creates_market: vec![],
                creates_limit: vec![
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                        price: Udec128::from_str("1.01").unwrap(),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: Udec128::from_str("852.485845").unwrap(),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Bid,
                        amount: NonZero::new_unchecked(Uint128::new(117304)),
                        price: Udec128::from_str("937.7344336").unwrap(),
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
                        price: Udec128::new(1),
                    },
                    CreateLimitOrderRequest {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction: Direction::Ask,
                        amount: NonZero::new_unchecked(Uint128::new(117304 * 2)),
                        price: Udec128::from_str("85248.71")
                            .unwrap()
                            .checked_inv()
                            .unwrap(),
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

#[test_case(
    vec![
        (
            vec![
                CreateLimitOrderRequest {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    direction: Direction::Ask,
                    amount: NonZero::new_unchecked(Uint128::new(100_000_000)),
                    price: Udec128::new(1),
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
        1 => (Direction::Ask, Udec128::new(1), Uint128::new(100_000_000), Uint128::new(100_000_000), 0),
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
                    price: Udec128::new(1),
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
        !1 => (Direction::Bid, Udec128::new(1), Uint128::new(100_000_000), Uint128::new(100_000_000), 0),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(2),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(2),
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
                    price: Udec128::new(1),
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
        1 => (Direction::Ask, Udec128::new(1), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
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
                    price: Udec128::new(2),
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
        1 => (Direction::Ask, Udec128::new(2), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
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
                    price: Udec128::new(1),
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
        !1 => (Direction::Bid, Udec128::new(1), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
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
                    price: Udec128::new(2),
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
        !1 => (Direction::Bid, Udec128::new(2), Uint128::new(150_000_000), Uint128::new(50_000_000), 0),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new_percent(200),
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
        2 => (Direction::Ask, Udec128::new_percent(200), Uint128::new(100_000_000), Uint128::new(75_000_000), 1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new_percent(200),
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
        2 => (Direction::Ask, Udec128::new_percent(200), Uint128::new(100_000_000), Uint128::new(75_000_000), 1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new_percent(50),
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
        !2 => (Direction::Bid, Udec128::new_percent(50), Uint128::new(100_000_000), Uint128::new(50_000_000), 1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new_percent(10),
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
        !2 => (Direction::Bid, Udec128::new_percent(10), Uint128::new(100_000_000), Uint128::new(87_500_000), 1),
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
                    price: Udec128::new(1),
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
        1 => (Direction::Ask, Udec128::new_percent(100), Uint128::new(100_000_000), Uint128::new(20_000_000), 0),
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
                    price: Udec128::new(1),
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
        !1 => (Direction::Bid, Udec128::new_percent(100), Uint128::new(100_000_000), Uint128::new(20_000_000), 0),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(2),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(2),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                    price: Udec128::new(1),
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
                        && queried_order.price == *price
                        && queried_order.amount == *amount
                        && queried_order.remaining == *remaining
                        && queried_order.user == users[*user_index].address()
                },
            )
        });
}
