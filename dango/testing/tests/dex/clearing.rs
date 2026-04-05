use {
    dango_genesis::{Contracts, DexOption, GenesisOption},
    dango_taxman::VOLUMES_BY_USER,
    dango_testing::{
        Preset, TestAccount, TestOption, TestSuite, setup_test_naive,
        setup_test_naive_with_custom_genesis,
    },
    dango_types::{
        config::AppConfig,
        constants::{dango, eth, usdc},
        dex::{
            self, CreateOrderRequest, Direction, Geometric, OrderId, PairParams, PairUpdate,
            PassiveLiquidity, Price, QueryOrdersRequest, QueryRestingOrderBookStateRequest,
            RestingOrderBookState,
        },
        oracle::{self, PriceSource},
        taxman,
    },
    grug::{
        Addressable, BalanceChange, Bounded, Coins, Denom, Duration, Fraction, Inner, Message,
        MultiplyFraction, NonEmpty, NonZero, NumberConst, Order, QuerierExt, ResultExt, Signer,
        StdResult, Timestamp, Udec128, Udec128_6, Uint128, btree_map, coins,
    },
    grug_app::NaiveProposalPreparer,
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
    },
    test_case::test_case,
};

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
        .map(|((direction, price, amount_base), signer)| {
            let price = Price::new(price);
            let amount_base = Uint128::new(amount_base);

            let (amount, funds) = match direction {
                Direction::Bid => {
                    let amount_quote = amount_base.checked_mul_dec_ceil(price).unwrap();
                    let funds = Coins::one(usdc::DENOM.clone(), amount_quote).unwrap();
                    (amount_quote, funds)
                },
                Direction::Ask => {
                    let funds = Coins::one(dango::DENOM.clone(), amount_base).unwrap();
                    (amount_base, funds)
                },
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        direction,
                        NonZero::new_unchecked(price),
                        NonZero::new_unchecked(amount),
                    )],
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

/// Submit a standard limit order. Used for volume tracking tests.
fn submit_standard_order(
    suite: &mut TestSuite<NaiveProposalPreparer>,
    user: &mut TestAccount,
    contracts: &Contracts,
    direction: Direction,
) {
    let funds = match direction {
        Direction::Bid => Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        Direction::Ask => Coins::one(dango::DENOM.clone(), 100_000_000).unwrap(),
    };

    suite
        .execute(
            user,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    direction,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                )],
                cancels: None,
            },
            funds,
        )
        .should_succeed();
}

#[test]
fn volume_tracking_works() {
    let (mut suite, accounts, _, contracts, _) = setup_test_naive(TestOption {
        // Taxman now tracks volumes with the granularity of 1 day. This test
        // was written before this change was introduced. To make this test work
        // with the smallest change, we simply set block time to 1 day, such that
        // each trade is 1 day difference in time.
        block_time: Duration::from_days(1),
        ..Default::default()
    });

    let mut user1_addr_1 = accounts.user1;
    let mut user1_addr_2 = user1_addr_1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap();

    let mut user2_addr_1 = accounts.user2;
    let mut user2_addr_2 = user2_addr_1
        .register_new_account(
            &mut suite,
            contracts.account_factory,
            Coins::one(dango::DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap();

    // Query volumes before, should be 0
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user1_addr_1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user2_addr_1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Submit a new order with user1 address 1
    submit_standard_order(&mut suite, &mut user1_addr_1, &contracts, Direction::Bid);

    // User2 submit an opposite matching order with address 1
    submit_standard_order(&mut suite, &mut user2_addr_1, &contracts, Direction::Ask);

    // Get timestamp after trade
    let timestamp_after_first_trade = suite.block.timestamp;

    // Submit a new order with user1 address 1
    submit_standard_order(&mut suite, &mut user1_addr_1, &contracts, Direction::Bid);

    // User2 submit an opposite matching order with address 1
    submit_standard_order(&mut suite, &mut user2_addr_1, &contracts, Direction::Ask);

    // Get timestamp after trade
    let timestamp_after_second_trade = suite.block.timestamp;

    // Query the volume for username user1, should be $200 = 200M USD microunits
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user1_addr_1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(200_000_000));

    // Query the volume for username user2, should be 200
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user2_addr_1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(200_000_000));

    // Submit a new order with user1 address 2
    submit_standard_order(&mut suite, &mut user1_addr_2, &contracts, Direction::Bid);

    // Submit a new opposite matching order with user2 address 2
    submit_standard_order(&mut suite, &mut user2_addr_2, &contracts, Direction::Ask);

    let timestamp_after_third_trade = suite.block.timestamp;

    // Query the volume for username user1, should be $300 = 300M USD microunits
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user1_addr_1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(300_000_000));

    // Query the volume for username user2, should be 300
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user2_addr_1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::new(300_000_000));

    // Query the volume for both usernames since timestamp after first trade, should be 200
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user1_addr_1.user_index(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::new(200_000_000));

    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user2_addr_1.user_index(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::new(200_000_000));

    // Query the volume for both usernames since timestamp after second trade, should be 100
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user1_addr_1.user_index(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Udec128::new(100_000_000));

    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: user2_addr_1.user_index(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Udec128::new(100_000_000));

    // Range over the stored volume data, ensure it's correct
    let storage = suite.contract_storage(contracts.taxman);
    let volumes_by_user = VOLUMES_BY_USER
        .range(&storage, None, None, Order::Ascending)
        .collect::<StdResult<BTreeMap<_, _>>>()
        .unwrap();
    assert_eq!(volumes_by_user, btree_map! {
        (user1_addr_1.user_index(), timestamp_after_first_trade)  => Udec128_6::new(100_000_000),
        (user1_addr_1.user_index(), timestamp_after_second_trade) => Udec128_6::new(200_000_000),
        (user1_addr_1.user_index(), timestamp_after_third_trade)  => Udec128_6::new(300_000_000),
        (user2_addr_1.user_index(), timestamp_after_first_trade)  => Udec128_6::new(100_000_000),
        (user2_addr_1.user_index(), timestamp_after_second_trade) => Udec128_6::new(200_000_000),
        (user2_addr_1.user_index(), timestamp_after_third_trade)  => Udec128_6::new(300_000_000),
    });
}

#[test]
fn volume_tracking_works_with_multiple_orders_from_same_user() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(TestOption {
        // See comment in `volume_tracking_works` for the rationale of this.
        block_time: Duration::from_days(1),
        ..Default::default()
    });

    // Submit two orders for DANGO/USDC and one for ETH/USDC with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::new(1)),
                        NonZero::new_unchecked(Uint128::new(100_000_000)),
                    ),
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::from_str("1.01").unwrap()),
                        NonZero::new_unchecked(Uint128::new(101_000_000)),
                    ),
                    CreateOrderRequest::new_limit(
                        eth::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::from_str("852.485845").unwrap()),
                        NonZero::new_unchecked(Uint128::new(100_000_000)), // ceil(117304 * 852.485845)
                    ),
                ],
                cancels: None,
            },
            coins! { usdc::DENOM.clone() => 301_000_000 },
        )
        .should_succeed();

    // Submit matching orders with user2
    suite
        .execute(
            &mut accounts.user2,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::new(1)),
                        NonZero::new_unchecked(Uint128::new(200_000_000)),
                    ),
                    CreateOrderRequest::new_limit(
                        eth::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::from_str("852.485845").unwrap()),
                        NonZero::new_unchecked(Uint128::new(117304)),
                    ),
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

    // Query the volume for username user1, should be:
    // - DANGO pair: 200_000_000
    // - ETH pair: 117_304 * 852.485845 = 99_999_999.56188
    // Sum = 200_000_000 + 99_999_999.56188 = 299_999_999.56188
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("299999999.56188").unwrap());

    // Query the volume for username user2, should be 300
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user2.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("299999999.56188").unwrap());

    // Query the volume for both usernames since timestamp after first trade, should be zero
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user1.user_index(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user2.user_index(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Submit new orders with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::new(1)),
                        NonZero::new_unchecked(Uint128::new(100_000_000)),
                    ),
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::from_str("1.01").unwrap()),
                        NonZero::new_unchecked(Uint128::new(101_000_000)),
                    ),
                    CreateOrderRequest::new_limit(
                        eth::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::from_str("852.485845").unwrap()),
                        NonZero::new_unchecked(Uint128::new(100_000_000)), // ceil(117304 * 852.485845)
                    ),
                    CreateOrderRequest::new_limit(
                        eth::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::from_str("937.7344336").unwrap()),
                        NonZero::new_unchecked(Uint128::new(110_000_000)), // ceil(117304 * 937.7344336)
                    ),
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
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::new(1)),
                        NonZero::new_unchecked(Uint128::new(300_000_000)),
                    ),
                    CreateOrderRequest::new_limit(
                        eth::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(
                            Price::from_str("85248.71").unwrap().checked_inv().unwrap(),
                        ),
                        NonZero::new_unchecked(Uint128::new(117304 * 2)),
                    ),
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
    // - Existing volume: 299_999_999.56188
    // - DANGO pair: 200_000_000
    // - ETH pair: 117_304 * 2 * 852.485845 = 199_999_999.12376
    // Sum = 299_999_999.56188 + 200_000_000 + 199_999_999.12376 = 699_999_998.68564
    // New volume = 200_000_000 + 199_999_999.12376 = 399_999_999.12376
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user1.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("699999998.68564").unwrap());

    // Query the volume for username user2, should be 700
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user2.user_index(),
            since: None,
        })
        .should_succeed_and_equal(Udec128::from_str("699999998.68564").unwrap());

    // Query the volume for both usernames since timestamp after second trade, should be zero
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user1.user_index(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user2.user_index(),
            since: Some(timestamp_after_second_trade),
        })
        .should_succeed_and_equal(Udec128::ZERO);

    // Query the volume for both usernames since timestamp after the first trade, should be 400
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user1.user_index(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::from_str("399999999.12376").unwrap());
    suite
        .query_wasm_smart(contracts.taxman, taxman::QueryVolumeByUserRequest {
            user: accounts.user2.user_index(),
            since: Some(timestamp_after_first_trade),
        })
        .should_succeed_and_equal(Udec128::from_str("399999999.12376").unwrap());
}

#[test_case(
    vec![
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
    }
    => panics "best bid price isn't available";
    "One limit ask, one market ask, no match"
)]
#[test_case(
    vec![
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
    }
    => panics "best ask price isn't available";
    "One limit bid, one market bid, no match"
)]
#[test_case(
    vec![
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(2)),
                    NonZero::new_unchecked(Uint128::new(200_000_000)), // 100_000_000 * 2
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(2)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(200_000_000)), // 100_000_000 * 2
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(2)),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 150_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(200_000_000)), // 100_000_000 * 2
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 150_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(2)),
                    NonZero::new_unchecked(Uint128::new(300_000_000)), // 150_000_000 * 2
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 300_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new_percent(105)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::new_percent(5)),
                    NonZero::new_unchecked(Uint128::new(157_500_000)), // 150_000_000 * 1.05
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 157_500_000,
            },
        )
    ],
    false,
    Udec128::ZERO,
    Udec128::ZERO,
    btree_map! {
        0 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(105_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(100_000_000), // partially filled limit order should stay on the book, half remaining
            usdc::DENOM.clone() => BalanceChange::Increased(52_500_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(150_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(157_500_000),
        },
    },
    btree_map! {
        OrderId::new(2) => (Direction::Ask, Udec128::new_percent(105), Uint128::new(100_000_000), Uint128::new(50_000_000), 1),
    };
    "Two limit asks different prices, one market bid, second limit partially filled, no fees, slippage not exceeded"
)]
#[test_case(
    vec![
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new_percent(50)),
                    NonZero::new_unchecked(Uint128::new(50_000_000)), // 100_000_000 * 0.5
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 50_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::new_percent(50)),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
            usdc::DENOM.clone() => BalanceChange::Decreased(50_000_000),
        },
        1 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Increased(50_000_000),
            usdc::DENOM.clone() => BalanceChange::Decreased(50_000_000),
        },
        2 => btree_map! {
            dango::DENOM.clone() => BalanceChange::Decreased(150_000_000),
            usdc::DENOM.clone() => BalanceChange::Increased(75_000_000),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::new_percent(99)),
                    NonZero::new_unchecked(Uint128::new(99_500_000)), // 50_000_000 * (1 + 0.99)
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 99_500_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::new_percent(99)),
                    NonZero::new_unchecked(Uint128::new(59_700_000)), // 30_000_000 * (1 + 0.99)
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 59_700_000,
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::new_percent(99)),
                    NonZero::new_unchecked(Uint128::new(50_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 50_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::new_percent(99)),
                    NonZero::new_unchecked(Uint128::new(30_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(2)),
                    NonZero::new_unchecked(Uint128::new(50_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 50_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(300_000_000)), // 150_000_000 * 2
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 300_000_000,
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(2)),
                    NonZero::new_unchecked(Uint128::new(200_000_000)), // 100_000_000 * 2
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 200_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
    BTreeMap::new()
    => panics "best ask price isn't available";
    "One limit ask price 1.0, one market bid, limit order smaller size, 1% maker fee, 5% taker fee, no slippage, limit order placed in same block as market order"
)]
#[test_case(
    vec![
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(150_000_000)),
                ),
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
    BTreeMap::new()
    => panics "best bid price isn't available";
    "One limit bid price 1.0, one market ask, limit order smaller size, 1% maker fee, 5% taker fee, no slippage, limit order placed in same block as market order"
)]
#[test_case(
    vec![
        (
            vec![
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(50_000_000)),
                ),
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(50_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(50_000_000)),
                ),
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(50_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(110_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 110_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
                CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(110_000_000)),
                ),
            ],
            coins! {
                dango::DENOM.clone() => 110_000_000,
            },
        )
    ],
    vec![
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
            ],
            coins! {
                usdc::DENOM.clone() => 100_000_000,
            },
        ),
        (
            vec![
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
                CreateOrderRequest::new_market(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    Bounded::new_unchecked(Udec128::ZERO),
                    NonZero::new_unchecked(Uint128::new(100_000_000)),
                ),
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
    limit_orders_and_funds: Vec<(Vec<CreateOrderRequest>, Coins)>,
    market_orders_and_funds: Vec<(Vec<CreateOrderRequest>, Coins)>,
    limits_and_markets_in_same_block: bool,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    expected_balance_changes: BTreeMap<usize, BTreeMap<Denom, BalanceChange>>,
    expected_limit_orders_after: BTreeMap<OrderId, (Direction, Udec128, Uint128, Uint128, usize)>,
) {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Set maker and taker fee rates
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
                creates: limit_orders,
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
                creates: market_orders,
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
    CreateOrderRequest::new_limit(
        eth::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_bps(1)),
        NonZero::new_unchecked(Uint128::new(50)), // 500000 * 0.0001
    ),
    coins! {
        usdc::DENOM.clone() => 50,
    },
    CreateOrderRequest::new_market(
        eth::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        Bounded::new_unchecked(Udec128::new_percent(8)),
        NonZero::new_unchecked(Uint128::new(9999)),
    ),
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
    ; "limit bid matched with market ask market size limiting factor market order does not get refunded"
)]
#[test_case(
    CreateOrderRequest::new_limit(
        eth::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Bid,
        NonZero::new_unchecked(Price::new_bps(1)),
        NonZero::new_unchecked(Uint128::new(1)), // ceil(9999 * 0.0001)
    ),
    coins! {
        usdc::DENOM.clone() => 1,
    },
    CreateOrderRequest::new_market(
        eth::DENOM.clone(),
        usdc::DENOM.clone(),
        Direction::Ask,
        Bounded::new_unchecked(Udec128::new_percent(8)),
        NonZero::new_unchecked(Uint128::new(500000)),
    ),
    coins! {
        eth::DENOM.clone() => 500000,
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Increased(9975),
        usdc::DENOM.clone() => BalanceChange::Decreased(1),
    },
    btree_map! {
        eth::DENOM.clone() => BalanceChange::Decreased(10000),
        usdc::DENOM.clone() => BalanceChange::Unchanged,
    }
    ; "limit bid matched with market ask limit size too small market order is consumed with zero output"
)]
fn market_order_matched_results_in_zero_output(
    limit_order: CreateOrderRequest,
    limit_funds: Coins,
    market_order: CreateOrderRequest,
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
                creates: vec![limit_order],
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
                creates: vec![market_order],
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
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(1000000)),
                )],
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
                creates: vec![CreateOrderRequest::new_limit(
                    dango::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Bid,
                    NonZero::new_unchecked(Price::new(1)),
                    NonZero::new_unchecked(Uint128::new(1000000)),
                )],
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
                    creates: vec![CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::new(1)),
                        NonZero::new_unchecked(Uint128::new(1000000)),
                    )],
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
                                creates: vec![CreateOrderRequest::new_market(
                                    dango::DENOM.clone(),
                                    usdc::DENOM.clone(),
                                    Direction::Bid,
                                    Bounded::new_unchecked(Udec128::ZERO),
                                    NonZero::new_unchecked(Uint128::new(1000000)),
                                )],
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
                                creates: vec![CreateOrderRequest::new_market(
                                    dango::DENOM.clone(),
                                    usdc::DENOM.clone(),
                                    Direction::Bid,
                                    Bounded::new_unchecked(Udec128::new_percent(5)),
                                    NonZero::new_unchecked(Uint128::new(1102500)),
                                )],
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
///
/// - id 1, limit ask, price 100, amount 1
/// - id 2, limit bid, price 101, amount 1
/// - id 3, market bid, price 100, amount 1
///
/// Since order 2 has the better price, it will be matched against 1.
/// Market order 3 will be popped out of the iterator, but not finding a match.
/// In this case, we need to handle the cancelation and refund of this order.
#[test]
fn refund_left_over_market_bid() {
    let (mut suite, mut accounts, _, contracts, _) =
        setup_test_naive_with_custom_genesis(Default::default(), GenesisOption {
            dex: DexOption {
                pairs: vec![PairUpdate {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Geometric(Geometric {
                            spacing: Udec128::new_percent(1),
                            ratio: Bounded::new_unchecked(Udec128::new(1)),
                            limit: 1,
                        }),
                        bucket_sizes: BTreeSet::new(),
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
                    },
                }],
            },
            ..Preset::preset_test()
        });

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

    // Block 1: we make it such that a mid price of 100 is recorded.
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::new(100)),
                        NonZero::new_unchecked(Uint128::new(2)),
                    ),
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::new(100)),
                        NonZero::new_unchecked(Uint128::new(100)),
                    ),
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
            best_ask_price: Some(Price::new(100)),
            mid_price: Some(Price::new(100)),
        });

    suite
        .balances()
        .record_many([&accounts.user1, &accounts.user2, &accounts.user3]);

    // Block 2: submit two orders:
    // - user 2 submits the limit order that will be matched;
    // - user 3 submits the market order that will be left over.
    // The limit order has slightly better price, so it has priority order the
    // market order.
    suite
        .make_block(vec![
            accounts
                .user2
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates: vec![CreateOrderRequest::new_limit(
                                    dango::DENOM.clone(),
                                    usdc::DENOM.clone(),
                                    Direction::Bid,
                                    NonZero::new_unchecked(Price::new(101)),
                                    NonZero::new_unchecked(Uint128::new(101)),
                                )],
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
                                creates: vec![CreateOrderRequest::new_market(
                                    dango::DENOM.clone(),
                                    usdc::DENOM.clone(),
                                    Direction::Bid,
                                    Bounded::new_unchecked(Udec128::ZERO),
                                    NonZero::new_unchecked(Uint128::new(100)),
                                )],
                                cancels: None,
                            },
                            coins! {
                                // Note: this should be 100. I typed this as 101 by mistake.
                                // But this shouldn't affect the test's outcome, since excess deposit should be refunded.
                                // Let's keep this as-is.
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
    let (mut suite, mut accounts, _, contracts, _) =
        setup_test_naive_with_custom_genesis(Default::default(), GenesisOption {
            dex: DexOption {
                pairs: vec![PairUpdate {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Geometric(Geometric {
                            spacing: Udec128::new_percent(1),
                            ratio: Bounded::new_unchecked(Udec128::new(1)),
                            limit: 1,
                        }),
                        bucket_sizes: BTreeSet::new(),
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
                    },
                }],
            },
            ..Preset::preset_test()
        });

    // Set maker and taker fee rates to 0 for simplicity
    // TODO: make this configurable in TestOptions
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
                creates: vec![
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Bid,
                        NonZero::new_unchecked(Price::new(100)),
                        NonZero::new_unchecked(Uint128::new(200)),
                    ),
                    CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        Direction::Ask,
                        NonZero::new_unchecked(Price::new(100)),
                        NonZero::new_unchecked(Uint128::new(1)),
                    ),
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
            best_bid_price: Some(Price::new(100)),
            best_ask_price: None,
            mid_price: Some(Price::new(100)),
        });

    suite
        .balances()
        .record_many([&accounts.user1, &accounts.user2, &accounts.user3]);

    // Block 2: submit two orders:
    // - user 2 submits the limit order that will be matched;
    // - user 3 submits the market order that will be left over.
    // The limit order has slightly better price, so it has priority order the
    // market order.
    suite
        .make_block(vec![
            accounts
                .user2
                .sign_transaction(
                    NonEmpty::new_unchecked(vec![
                        Message::execute(
                            contracts.dex,
                            &dex::ExecuteMsg::BatchUpdateOrders {
                                creates: vec![CreateOrderRequest::new_limit(
                                    dango::DENOM.clone(),
                                    usdc::DENOM.clone(),
                                    Direction::Ask,
                                    NonZero::new_unchecked(Price::new(99)),
                                    NonZero::new_unchecked(Uint128::new(1)),
                                )],
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
                                creates: vec![CreateOrderRequest::new_market(
                                    dango::DENOM.clone(),
                                    usdc::DENOM.clone(),
                                    Direction::Ask,
                                    Bounded::new_unchecked(Udec128::ZERO),
                                    NonZero::new_unchecked(Uint128::new(1)),
                                )],
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

#[test]
fn resting_order_book_is_updated_correctly_orders_remain_on_both_sides() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let txs = vec![
        accounts
            .user1
            .sign_transaction(
                NonEmpty::new_unchecked(vec![
                    Message::execute(
                        contracts.dex,
                        &dex::ExecuteMsg::BatchUpdateOrders {
                            creates: vec![CreateOrderRequest::new_limit(
                                dango::DENOM.clone(),
                                usdc::DENOM.clone(),
                                Direction::Ask,
                                NonZero::new_unchecked(Price::new(100)),
                                NonZero::new_unchecked(Uint128::new(1000000)),
                            )],
                            cancels: None,
                        },
                        coins! {
                            dango::DENOM.clone() => 1000000,
                        },
                    )
                    .unwrap(),
                ]),
                &suite.chain_id,
                100_000,
            )
            .unwrap(),
        accounts
            .user2
            .sign_transaction(
                NonEmpty::new_unchecked(vec![
                    Message::execute(
                        contracts.dex,
                        &dex::ExecuteMsg::BatchUpdateOrders {
                            creates: vec![CreateOrderRequest::new_limit(
                                dango::DENOM.clone(),
                                usdc::DENOM.clone(),
                                Direction::Bid,
                                NonZero::new_unchecked(Price::new(99)),
                                NonZero::new_unchecked(Uint128::new(1000000 * 99)),
                            )],
                            cancels: None,
                        },
                        coins! {
                            usdc::DENOM.clone() => 1000000 * 99,
                        },
                    )
                    .unwrap(),
                ]),
                &suite.chain_id,
                100_000,
            )
            .unwrap(),
    ];

    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    suite
        .query_wasm_smart(contracts.dex, QueryRestingOrderBookStateRequest {
            base_denom: dango::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed_and_equal(RestingOrderBookState {
            best_bid_price: Some(Price::new(99)),
            best_ask_price: Some(Price::new(100)),
            mid_price: Some(Price::new_permille(99500)),
        });
}
