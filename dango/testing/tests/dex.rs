use {
    dango_testing::setup_test_naive,
    dango_types::{
        constants::{ATOM_DENOM, DANGO_DENOM, ETH_DENOM, USDC_DENOM},
        dex::{
            self, CurveInvariant, Direction, OrderId, OrderSubmissionInfo, Pool,
            QueryOrdersByUserRequest, QueryOrdersRequest, Swap,
        },
    },
    grug::{
        btree_map, coins, Addressable, BalanceChange, Coin, CoinPair, Coins, Denom, Inner, Message,
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
            &dex::ExecuteMsg::BatchSubmitOrders(vec![OrderSubmissionInfo {
                base_denom: ATOM_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::new(100),
                price: Udec128::new(1),
            }]),
            Coins::one(USDC_DENOM.clone(), 100).unwrap(),
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
                &dex::ExecuteMsg::BatchSubmitOrders(vec![OrderSubmissionInfo {
                    base_denom: DANGO_DENOM.clone(),
                    quote_denom: USDC_DENOM.clone(),
                    direction,
                    amount,
                    price,
                }]),
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
            &dex::ExecuteMsg::BatchSubmitOrders(vec![OrderSubmissionInfo {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                direction: Direction::Bid,
                amount: Uint128::new(100),
                price: Udec128::new(1),
            }]),
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
        &dex::ExecuteMsg::BatchSubmitOrders(vec![OrderSubmissionInfo {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(100),
            price: Udec128::new(1),
        }]),
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

#[test]
fn only_owner_can_create_passive_pool() {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/lp/dangousdc").unwrap();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::CreatePassivePool {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                curve_type: CurveInvariant::Xyk,
                lp_denom: lp_denom.clone(),
                swap_fee: Udec128::ZERO,
                tick_size: Udec128::ONE,
                order_depth: Uint128::ONE,
            },
            Coins::new(),
        )
        .should_fail_with_error("Only the owner can create a passive pool");

    suite.balances().record(contracts.dex.address());

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::CreatePassivePool {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                curve_type: CurveInvariant::Xyk,
                lp_denom: lp_denom.clone(),
                swap_fee: Udec128::ZERO,
                tick_size: Udec128::ONE,
                order_depth: Uint128::ONE,
            },
            coins! { USDC_DENOM.clone() => 100, DANGO_DENOM.clone() => 100 },
        )
        .should_succeed();

    suite
        .balances()
        .should_change(contracts.dex.address(), btree_map! {
            USDC_DENOM.clone() => BalanceChange::Increased(100),
            DANGO_DENOM.clone() => BalanceChange::Increased(100),
        });

    suite
        .query_wasm_smart(contracts.dex, dango_types::dex::QueryPassivePoolRequest {
            lp_denom,
        })
        .should_succeed_and_equal(Pool {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            curve_type: CurveInvariant::Xyk,
            reserves: CoinPair::new_unchecked(
                Coin {
                    denom: DANGO_DENOM.clone(),
                    amount: Uint128::from(100),
                },
                Coin {
                    denom: USDC_DENOM.clone(),
                    amount: Uint128::from(100),
                },
            ),
            swap_fee: Udec128::ZERO,
            tick_size: Udec128::ONE,
            order_depth: Uint128::ONE,
        });
}

#[test_case(
    coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 100,
    },
    Uint128::new(100) ; "provision at pool ratio"
)]
#[test_case(
    coins! {
        DANGO_DENOM.clone() => 50,
        USDC_DENOM.clone() => 50,
    },
    Uint128::new(50) ; "provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 50,
    },
    Uint128::new(73) ; "provision at different ratio"
)]
fn provide_liquidity(provision: Coins, expected_lp_balance: Uint128) {
    let lp_denom = Denom::try_from("dex/lp/dangousdc").unwrap();
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    // Record the users initial balances.
    suite
        .balances()
        .record_many(accounts.users().map(|user| user.address()));

    // Create a passive pool.
    let initial_reserves = coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 100,
    };
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::CreatePassivePool {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                curve_type: CurveInvariant::Xyk,
                lp_denom: lp_denom.clone(),
                swap_fee: Udec128::ZERO,
                tick_size: Udec128::ONE,
                order_depth: Uint128::ONE,
            },
            initial_reserves.clone(),
        )
        .should_succeed();

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
                lp_denom: lp_denom.clone(),
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
            coins! { lp_denom.clone() => expected_lp_balance},
            provision.clone(),
        ),
    );

    // Check that the reserves in pool object were updated correctly.
    suite
        .query_wasm_smart(contracts.dex, dango_types::dex::QueryPassivePoolRequest {
            lp_denom: lp_denom.clone(),
        })
        .should_succeed_and_equal(Pool {
            base_denom: DANGO_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            curve_type: CurveInvariant::Xyk,
            reserves: expected_pool_balances
                .insert_many(provision.clone())
                .unwrap()
                .clone()
                .try_into()
                .unwrap(),
            swap_fee: Udec128::ZERO,
            tick_size: Udec128::ONE,
            order_depth: Uint128::ONE,
        });
}

#[test_case(
    Uint128::new(100),
    coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone()  => 100,
    } ; "withdrawa all"
)]
#[test_case(
    Uint128::new(50),
    coins! {
        DANGO_DENOM.clone() => 50,
        USDC_DENOM.clone()  => 50,
    } ; "withdraw half"
)]
fn withdraw_liquidity(withdraw_amount: Uint128, expected_funds_returned: Coins) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/lp/dangousdc").unwrap();

    // Create a passive pool.
    let initial_funds = coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone()  => 100,
    };
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::CreatePassivePool {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                curve_type: CurveInvariant::Xyk,
                lp_denom: lp_denom.clone(),
                swap_fee: Udec128::ZERO,
                tick_size: Udec128::ONE,
                order_depth: Uint128::ONE,
            },
            initial_funds.clone(),
        )
        .should_succeed();

    // provide liquidity
    let provided_funds = coins! {
        DANGO_DENOM.clone() => 100,
        USDC_DENOM.clone() => 100,
    };
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                lp_denom: lp_denom.clone(),
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
            &dex::ExecuteMsg::WithdrawLiquidity {},
            coins! {
                lp_denom.clone() => withdraw_amount,
            },
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    suite.balances().should_change(
        accounts.user1.address(),
        balance_changes_from_coins(
            expected_funds_returned.clone(),
            coins! { lp_denom.clone() => withdraw_amount },
        ),
    );

    // Assert that the dex balance has decreased by the expected amount.
    suite.balances().should_change(
        contracts.dex.address(),
        balance_changes_from_coins(Coins::new(), expected_funds_returned.clone()),
    );

    // Assert pool reserves are updated correctly
    suite
        .query_wasm_smart(contracts.dex, dango_types::dex::QueryPassivePoolRequest {
            lp_denom: lp_denom.clone(),
        })
        .should_succeed_and(|pool| {
            Coins::from(pool.reserves.clone())
                == *initial_funds
                    .clone()
                    .insert_many(provided_funds)
                    .unwrap()
                    .deduct_many(expected_funds_returned)
                    .unwrap()
        });
}

#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "swap amount in no fee"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::new_permille(1),
    coins! {
        USDC_DENOM.clone() => 499500,
    } ; "swap amount in with 0.1% fee"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 999999,
    },
    coins! {
        ETH_DENOM.clone() => 999999,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } => panics "insufficient funds" ; "swap amount in insufficient funds"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 2000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "swap amount in excessive funds reimbursed"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "swap amount out no fee"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(499500),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::new_permille(1),
    coins! {
        USDC_DENOM.clone() => 499500,
    } ; "swap amount out 0.1% fee"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(1000000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::new_permille(1),
    coins! {
        USDC_DENOM.clone() => 499500,
    } => panics "insufficient liquidity" ; "swap amount out insufficient liquidity"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 999999,
    },
    coins! {
        ETH_DENOM.clone() => 999999,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } => panics "insufficient funds" ; "swap amount out insufficient funds"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 2000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "swap amount out excessive funds reimbursed"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: None,
        },
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(500000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Coins::new(),
    Udec128::ZERO,
    Coins::new() ; "multiple swaps swap amount out then swap back with swap amount in"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        // Due to path independence, this should be equivalent to a single swap amount in of 1000000 ETH
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(250000),
            slippage: None,
        },
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(750000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "multiple swaps two consecutive swap amount in"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        // Due to path independence, this should be equivalent to a single swap
        // amount out of 500000 USDC. Ends up using 1 more ETH due to rounding.
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(250000),
            slippage: None,
        },
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(250000),
            slippage: None,
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000001, // 1 more due to rounding
    },
    coins! {
        ETH_DENOM.clone() => 1000001, // 1 more due to rounding
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "multiple swaps two consecutive swap amount out"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: Some(dex::SlippageControl::MinimumOut(Uint128::new(500000))),
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "swap amount in no fee minimum out not exceeded"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: Some(dex::SlippageControl::MinimumOut(Uint128::new(500001))),
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } => panics "slippage tolerance exceeded" ; "swap amount in no fee minimum out exceeded"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: ETH_DENOM.clone(),
            quote_denom: USDC_DENOM.clone(),
            direction: Direction::Ask,
            amount: Uint128::new(1000000),
            slippage: Some(dex::SlippageControl::MaximumIn(Uint128::new(1000000))),
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } => panics "maximum in is only supported for direction: bid" ; "swap amount in no fee maximum in"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: Some(dex::SlippageControl::MaximumIn(Uint128::new(1000000))),
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } ; "swap amount out no fee maximum in not exceeded"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: Some(dex::SlippageControl::MaximumIn(Uint128::new(999999))),
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } => panics "slippage tolerance exceeded" ; "swap amount out no fee maximum in exceeded"
)]
#[test_case(
    coins! {
        ETH_DENOM.clone() => 1000000,
        USDC_DENOM.clone() => 1000000,
    },
    vec![
        Swap {
            base_denom: USDC_DENOM.clone(),
            quote_denom: ETH_DENOM.clone(),
            direction: Direction::Bid,
            amount: Uint128::new(500000),
            slippage: Some(dex::SlippageControl::MinimumOut(Uint128::new(500000))),
        }
    ],
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    coins! {
        ETH_DENOM.clone() => 1000000,
    },
    Udec128::ZERO,
    coins! {
        USDC_DENOM.clone() => 500000,
    } => panics "minimum out is only supported for direction: ask" ; "swap amount out no fee minimum out exceeded"
)]
// TODO: Tests
// - Swap direction ask SlippageControl::PriceLimit
// - Swap direction bid SlippageControl::PriceLimit
fn batch_swap(
    pool_liquidity: Coins,
    swaps: Vec<Swap>,
    swap_funds: Coins,
    expected_funds_used: Coins,
    swap_fee: Udec128,
    expected_out: Coins,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/lp/ethusdc").unwrap();

    // Create a passive pool.
    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::CreatePassivePool {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                curve_type: CurveInvariant::Xyk,
                lp_denom: lp_denom.clone(),
                swap_fee,
                tick_size: Udec128::ONE,
                order_depth: Uint128::ONE,
            },
            pool_liquidity.clone(),
        )
        .should_succeed();

    // Record user and dex balances
    suite
        .balances()
        .record_many(vec![accounts.user1.address(), contracts.dex.address()]);

    // User swaps
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchSwap { swaps },
            swap_funds.clone(),
        )
        .should_succeed();

    // Assert that the user's balances have changed as expected.
    suite.balances().should_change(
        accounts.user1.address(),
        balance_changes_from_coins(expected_out.clone(), expected_funds_used.clone()),
    );

    // Assert that the dex balance has changed by the expected amount.
    suite.balances().should_change(
        contracts.dex.address(),
        balance_changes_from_coins(expected_funds_used.clone(), expected_out.clone()),
    );

    // Query pool and assert that the reserves are updated correctly
    let expected_pool_reserves = pool_liquidity
        .clone()
        .deduct_many(expected_out)
        .unwrap()
        .insert_many(expected_funds_used)
        .unwrap()
        .clone();

    suite
        .query_wasm_smart(contracts.dex, dango_types::dex::QueryPassivePoolRequest {
            lp_denom: lp_denom.clone(),
        })
        .should_succeed_and(|pool| Coins::from(pool.reserves.clone()) == expected_pool_reserves);
}

// TODO: test multiple pools
#[test]
fn batch_swap_multiple_pools() {}

fn balance_changes_from_coins(
    increases: Coins,
    decreases: Coins,
) -> BTreeMap<Denom, BalanceChange> {
    let mut changes: BTreeMap<Denom, BalanceChange> = increases
        .into_iter()
        .map(|Coin { denom, amount }| {
            (denom.clone(), BalanceChange::Increased(amount.into_inner()))
        })
        .collect();
    changes.extend(decreases.into_iter().map(|Coin { denom, amount }| {
        (denom.clone(), BalanceChange::Decreased(amount.into_inner()))
    }));
    changes
}

#[test_case(
    CurveInvariant::Xyk,
    Udec128::ONE,
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 200 * 10000000,
    },
    btree_map! { // Map from order_id to expected (price, amount)
        !2  => (Udec128::new_percent(19900), 50251),
        !4  => (Udec128::new_percent(19800), 50759),
        !6  => (Udec128::new_percent(19700), 51274),
        !8  => (Udec128::new_percent(19600), 51797),
        !10 => (Udec128::new_percent(19500), 52329),
        !12 => (Udec128::new_percent(19400), 52868),
        !14 => (Udec128::new_percent(19300), 53416),
        !16 => (Udec128::new_percent(19200), 53972),
        !18 => (Udec128::new_percent(19100), 54538),
        !20 => (Udec128::new_percent(19000), 55112),
        3   => (Udec128::new_percent(20100), 49751),
        5   => (Udec128::new_percent(20200), 49259),
        7   => (Udec128::new_percent(20300), 48773),
        9   => (Udec128::new_percent(20400), 48295),
        11  => (Udec128::new_percent(20500), 47824),
        13  => (Udec128::new_percent(20600), 47360), 
        15  => (Udec128::new_percent(20700), 46902),
        17  => (Udec128::new_percent(20800), 46451),
        19  => (Udec128::new_percent(20900), 46007),
        21  => (Udec128::new_percent(21000), 45568),
    } ; "xyk pool balance 1:200")]
#[test_case(
    CurveInvariant::Xyk,
    Udec128::new_percent(1),
    coins! {
        ETH_DENOM.clone() => 10000000,
        USDC_DENOM.clone() => 10000000,
    },
    btree_map! { // Map from order_id to expected (price, amount)
        !2  => (Udec128::new_percent(99), 101010),
        !4  => (Udec128::new_percent(98), 103072),
        !6  => (Udec128::new_percent(97), 105197),
        !8  => (Udec128::new_percent(96), 107388),
        !10 => (Udec128::new_percent(95), 109649),
        !12 => (Udec128::new_percent(94), 111982),
        !14 => (Udec128::new_percent(93), 114390),
        !16 => (Udec128::new_percent(92), 116877),
        !18 => (Udec128::new_percent(91), 119446),
        !20 => (Udec128::new_percent(90), 122100),
        3   => (Udec128::new_percent(101), 99010),
        5   => (Udec128::new_percent(102), 97069),
        7   => (Udec128::new_percent(103), 95184),
        9   => (Udec128::new_percent(104), 93353),
        11  => (Udec128::new_percent(105), 91575),
        13  => (Udec128::new_percent(106), 89847), 
        15  => (Udec128::new_percent(107), 88168),
        17  => (Udec128::new_percent(108), 86535),
        19  => (Udec128::new_percent(109), 84947),
        21  => (Udec128::new_percent(110), 83403),
    } ; "xyk pool balance 1:1")]
fn curve_on_orderbook(
    curve_invariant: CurveInvariant,
    tick_size: Udec128,
    pool_liquidity: Coins,
    expected_orders: BTreeMap<OrderId, (Udec128, u128)>,
) {
    let (mut suite, mut accounts, _, contracts) = setup_test_naive();

    let lp_denom = Denom::try_from("dex/lp/ethusdc").unwrap();

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::CreatePassivePool {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                curve_type: curve_invariant,
                lp_denom: lp_denom.clone(),
                swap_fee: Udec128::ZERO,
                tick_size,
                order_depth: Uint128::from(10),
            },
            pool_liquidity.clone(),
        )
        .should_succeed();

    suite
        .query_wasm_smart(contracts.dex, QueryOrdersByUserRequest {
            user: contracts.dex.address(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and(|orders| {
            println!("orders: {:?}", orders);
            assert_eq!(orders.len(), 20);
            for (order_id, (price, amount)) in expected_orders {
                let order = orders.get(&order_id).unwrap();
                assert_eq!(order.price, price);
                assert!(order.amount.into_inner().abs_diff(amount) <= 1);
            }
            true
        });
}


