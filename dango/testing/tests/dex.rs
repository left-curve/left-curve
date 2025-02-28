use {
    dango_testing::setup_test_naive,
    dango_types::{
        constants::{ATOM_DENOM, DANGO_DENOM, ETH_DENOM, USDC_DENOM, XRP_DENOM},
        dex::{
            self, CurveInvariant, Direction, OrderId, PairParams, PairUpdate,
            QueryOrdersByPairRequest, QueryOrdersRequest,
        },
    },
    grug::{
        btree_map, coins, Addressable, BalanceChange, Bounded, Coin, Coins, Denom, Inner, Message,
        MultiplyFraction, NonEmpty, NumberConst, QuerierExt, ResultExt, Signer, StdResult, Udec128,
        Uint128,
    },
    std::collections::{BTreeMap, BTreeSet},
    test_case::test_case,
};

#[test]
fn cannot_submit_orders_in_non_existing_pairs() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SubmitOrder {
                base_denom: ATOM_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::new(100),
                price: Udec128::new(1),
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
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(200),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(200),
        },
        5 => btree_map! {
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
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        5 => btree_map! {
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
        !1 =>  5,
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(10),
            USDC_DENOM.clone()  => BalanceChange::Decreased(175),
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(5),   // half filled
            USDC_DENOM.clone()  => BalanceChange::Decreased(188), // -200 deposit, +12 refund
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(175),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Unchanged,
        },
        !6 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(5),
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
        !1 => 10,
        !2 => 10,
         5 => 10,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(20),
            USDC_DENOM.clone()  => BalanceChange::Decreased(450), // -600 deposit, +150 refund
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(225),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(225),
        },
        5 => btree_map! {
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
        !1 => 10,
        !2 => 10,
         5 =>  5,
    },
    btree_map! {
        !0 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Increased(25),
            USDC_DENOM.clone()  => BalanceChange::Decreased(688), // -750 deposit, +62 refund
        },
        !1 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(200),
        },
        !2 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Unchanged,
            USDC_DENOM.clone()  => BalanceChange::Decreased(100),
        },
        3 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(275),
        },
        4 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(275),
        },
        5 => btree_map! {
            DANGO_DENOM.clone() => BalanceChange::Decreased(10),
            USDC_DENOM.clone()  => BalanceChange::Increased(137), // refund: floor(5 * 27.5) = 137
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

    // Find which accounts will submit the orders, so we can track their balances.
    let users_by_order_id = orders_to_submit
        .iter()
        .zip(accounts.users())
        .enumerate()
        .map(|(order_id, ((direction, ..), signer))| {
            let order_id = order_id as OrderId;
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
                &dex::ExecuteMsg::SubmitOrder {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction,
                    amount,
                    price,
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

#[test]
fn cancel_order() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Record the user's balance
    suite.balances().record(accounts.user1.address());

    // Add order to the order book
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SubmitOrder {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::new(100),
                price: Udec128::new(1),
            },
            grug::coins! {
                USDC_DENOM.clone() => 100
            },
        )
        .should_succeed();

    // Cancel the order
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::CancelOrders {
                order_ids: dex::OrderIds::Some(BTreeSet::from([!0])),
            },
            coins! { DANGO_DENOM.clone() => 1 },
        )
        .should_succeed();

    // Check that the user balance has not changed
    suite.balances().should_change(
        accounts.user1.address(),
        btree_map! { USDC_DENOM.clone() => BalanceChange::Unchanged },
    );

    // Check that order does not exist
    suite
        .query_wasm_smart(contracts.dex, QueryOrdersRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and(BTreeMap::is_empty);
}

#[test]
fn submit_and_cancel_order_in_same_block() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Record the user's balance
    suite.balances().record(accounts.user1.address());

    // Build and sign a transaction with two messages: submit an order and cancel the order
    let submit_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::SubmitOrder {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        },
        coins! { USDC_DENOM.clone() => 100 },
    )
    .unwrap();

    let cancel_order_msg = Message::execute(
        contracts.dex,
        &dex::ExecuteMsg::CancelOrders {
            order_ids: dex::OrderIds::Some(BTreeSet::from([!0])),
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
        !0 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        !1 => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        2 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
        3 => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
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
        !4 => (Direction::Bid, Udec128::new(20), Uint128::new(10)),
        5 => (Direction::Ask, Udec128::new(25), Uint128::new(10)),
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
        !0 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        !1 => (Direction::Bid, Udec128::new(10), Uint128::new(10)),
        2 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
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
    Some(2),
    None,
    btree_map! {
        3 => (Direction::Ask, Udec128::new(50), Uint128::new(10)),
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
    Some(!1),
    Some(2),
    btree_map! {
        !0 => (Direction::Bid, Udec128::new(30), Uint128::new(10)),
        2 => (Direction::Ask, Udec128::new(40), Uint128::new(10)),
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
                &dex::ExecuteMsg::SubmitOrder {
                    base_denom,
                    quote_denom,
                    direction,
                    amount,
                    price,
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
