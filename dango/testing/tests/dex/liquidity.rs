use {
    dango_dex::MINIMUM_LIQUIDITY,
    dango_oracle::PYTH_PRICES,
    dango_testing::setup_test_naive,
    dango_types::{
        config::AppConfig,
        constants::{dango, eth, usdc, xrp},
        dex::{
            self, CancelOrderRequest, CreateOrderRequest, Direction, Geometric, OrderId,
            PairParams, PairUpdate, PassiveLiquidity, Price, QueryLiquidityDepthRequest,
            QueryOrdersRequest, QueryReserveRequest, Xyk,
        },
        oracle::{self, PrecisionlessPrice, PriceSource},
    },
    grug::{
        Addressable, BalanceChange, Bounded, Coin, CoinPair, Coins, Denom, Fraction, Inner,
        Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, QuerierExt, ResultExt, Signer,
        StdError, StdResult, Timestamp, Udec128, Udec128_6, Uint128, btree_map, btree_set,
        coin_pair, coins,
    },
    pyth_types::constants::USDC_USD_ID,
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
    },
    test_case::test_case,
};

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
                    pool_type: PassiveLiquidity::Xyk(Xyk {
                        spacing: Udec128::new_bps(1),
                        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                        limit: 10,
                    }),
                    bucket_sizes: BTreeSet::new(),
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                    min_order_size_quote: Uint128::ZERO,
                    min_order_size_base: Uint128::ZERO,
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
                    pool_type: PassiveLiquidity::Xyk(Xyk {
                        spacing: Udec128::new_bps(1),
                        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                        limit: 10,
                    }),
                    bucket_sizes: BTreeSet::new(),
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_permille(5)),
                    min_order_size_quote: Uint128::ZERO,
                    min_order_size_base: Uint128::ZERO,
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(100_000_000)
    ; "provision at pool ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(50_000_000)
    ; "provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(1)),
        (usdc::DENOM.clone(), Udec128::new(1)),
    ],
    Uint128::new(72_965_238)
    ; "provision at different ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 100,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric(Geometric {
        spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(300_000_000)
    ; "geometric pool provision at pool ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric(Geometric {
        spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(150_000_000)
    ; "geometric pool provision at half pool balance same ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone() => 50,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric(Geometric {
        spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(249_909_089)
    ; "geometric pool provision at different ratio"
)]
#[test_case(
    coins! {
        dango::DENOM.clone() => 50,
        usdc::DENOM.clone() => 100,
    },
    Udec128::new_permille(5),
    PassiveLiquidity::Geometric(Geometric {
        spacing: Udec128::ONE,
        ratio: Bounded::new_unchecked(Udec128::new_percent(50)),
        limit: 10,
    }),
    vec![
        (dango::DENOM.clone(), Udec128::new(2_000_000)),
        (usdc::DENOM.clone(), Udec128::new(1_000_000)),
    ],
    Uint128::new(199_899_999)
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
                            bucket_sizes: BTreeSet::new(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee),
                            pool_type,
                            min_order_size_quote: Uint128::ZERO,
                            min_order_size_base: Uint128::ZERO,
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
                minimum_output: None,
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
                minimum_output: None,
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
                            bucket_sizes: BTreeSet::new(),
                            swap_fee_rate: pair_params.swap_fee_rate,
                            pool_type: PassiveLiquidity::Geometric(Geometric {
                                spacing: Udec128::ONE,
                                ratio: Bounded::new_unchecked(Udec128::ONE),
                                limit: 10,
                            }),
                            min_order_size_quote: Uint128::ZERO,
                            min_order_size_base: Uint128::ZERO,
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
                minimum_output: None,
            },
            coins! {
                dango::DENOM.clone() => 100_000,
                usdc::DENOM.clone() => 100_000,
            },
        )
        .should_fail_with_error(StdError::data_not_found::<PrecisionlessPrice>(
            PYTH_PRICES.path(USDC_USD_ID.id).storage_key(),
        ));
}

#[test_case(
    Uint128::new(100_000_000),
    Udec128::new_permille(5),
    coins! {
        dango::DENOM.clone() => 100,
        usdc::DENOM.clone()  => 100,
    };
    "withdrawa all"
)]
#[test_case(
    Uint128::new(50_000_000),
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
                            bucket_sizes: BTreeSet::new(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee),
                            pool_type: pair_params.pool_type.clone(),
                            min_order_size_quote: Uint128::ZERO,
                            min_order_size_base: Uint128::ZERO,
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
                minimum_output: None,
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
                minimum_output: None,
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
                minimum_output: None,
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

#[test_case(
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Bid,
                NonZero::new_unchecked(Price::new_percent(20100)),
                NonZero::new_unchecked(Uint128::from(49751 * 201)),
            ),
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 49751 * 201,
        },
    ],
    btree_map! {
        // Why the order ID should be `!21`? Because:
        // - The one block where we provide liquidity, the auction generates 10
        //   passive orders on each side of the book. They take up order IDs 1-20.
        // - The next block, we place a new order. It gets order ID 21.
        // - Because the order is a bid, it's bitwise NOT-ed, so `!21`.
        OrderId::new(!21) => (Udec128::new_percent(20100), Udec128::new(49751), Direction::Bid),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Bid,
                NonZero::new_unchecked(Price::new_percent(20100)),
                NonZero::new_unchecked(Uint128::from(49751 * 201)),
            ),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Bid,
                NonZero::new_unchecked(Price::new_percent(20200)),
                NonZero::new_unchecked(Uint128::from(47783 * 202)),
            ),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Bid,
                NonZero::new_unchecked(Price::new_percent(20300)),
                NonZero::new_unchecked(Uint128::from(157784 * 203)),
            ),
        ],
    ],
    vec![
        coins! {
            usdc::DENOM.clone() => 157784 * 203,
        },
    ],
    btree_map! {
        OrderId::new(!21) => (Udec128::new_percent(20300), Udec128::new(10000), Direction::Bid),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Ask,
                NonZero::new_unchecked(Price::new_percent(19900)),
                NonZero::new_unchecked(Uint128::from(50251)),
            ),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Ask,
                NonZero::new_unchecked(Price::new_percent(19900)),
                NonZero::new_unchecked(Uint128::from(30000)),
            ),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_permille(5),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Ask,
                NonZero::new_unchecked(Price::new_percent(19900)),
                NonZero::new_unchecked(Uint128::from(60251)),
            ),
        ],
    ],
    vec![
        coins! {
            eth::DENOM.clone() => 60251,
        },
    ],
    btree_map! {
        OrderId::new(21) => (Udec128::new_percent(19900), Udec128::new(10000), Direction::Ask),
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
    PassiveLiquidity::Xyk(Xyk {
        spacing: Udec128::ONE,
        reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
        limit: 10,
    }),
    Udec128::new_percent(1),
    coins! {
        eth::DENOM.clone() => 10000000,
        usdc::DENOM.clone() => 200 * 10000000,
    },
    vec![
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Ask,
                NonZero::new_unchecked(Price::new_percent(19800)),
                NonZero::new_unchecked(Uint128::from(162284)),
            ),
        ],
        vec![
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                Direction::Bid,
                NonZero::new_unchecked(Price::new_percent(20200)),
                NonZero::new_unchecked(Uint128::from(157784 * 202)),
            ),
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
    orders: Vec<Vec<CreateOrderRequest>>,
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
                            bucket_sizes: BTreeSet::new(),
                            swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                            min_order_size_quote: Uint128::ZERO,
                            min_order_size_base: Uint128::ZERO,
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
                minimum_output: None,
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
            // `expected_orders_after_clearing` contains only the expected
            // orders from the users, so we filter off the passive pool orders.
            assert_eq!(
                orders
                    .iter()
                    .filter(|(_, order)| order.user != contracts.dex)
                    .count(),
                expected_orders_after_clearing.len()
            );
            for (order_id, (price, remaining, direction)) in expected_orders_after_clearing {
                let order = orders.get(&order_id).unwrap();
                assert_eq!(order.price, price.convert_precision().unwrap());
                assert_eq!(order.remaining, remaining.convert_precision().unwrap());
                assert_eq!(order.direction, direction);
            }
            true
        });
}

#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Ask, Price::new(52), Uint128::new(1)),
    ],
    None,
    Price::new(1),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: None,
        ask_depth: Some(vec![
            (Price::new(52), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(52),
            }),
        ]),
    },
    None;
    "no bid depth, one ask at 52, bucket size 1"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Ask, Price::new(52), Uint128::new(1)),
    ],
    None,
    Price::new(10),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: None,
        ask_depth: Some(vec![
            (Price::new(60), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(52),
            }),
        ]),
    },
    None;
    "no bid depth, one ask at 52, bucket size 10"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(48), Uint128::new(1)),
    ],
    None,
    Price::new(1),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(48), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(48),
            }),
        ]),
        ask_depth: None,
    },
    None;
    "no ask depth, one bid at 48, bucket size 1"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(48), Uint128::new(1)),
    ],
    None,
    Price::new(10),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(40), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(48),
            }),
        ]),
        ask_depth: None,
    },
    None;
    "no ask depth, one bid at 48, bucket size 10"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(49), Uint128::new(1)),
        (Direction::Bid, Price::new(48), Uint128::new(2)),
        (Direction::Bid, Price::new(47), Uint128::new(3)),
        (Direction::Bid, Price::new(46), Uint128::new(4)),
        (Direction::Bid, Price::new(45), Uint128::new(5)),
        (Direction::Bid, Price::new(44), Uint128::new(6)),
        (Direction::Bid, Price::new(43), Uint128::new(7)),
        (Direction::Bid, Price::new(42), Uint128::new(8)),
        (Direction::Bid, Price::new(41), Uint128::new(9)),
        (Direction::Bid, Price::new(40), Uint128::new(10)),
        (Direction::Ask, Price::new(51), Uint128::new(1)),
        (Direction::Ask, Price::new(52), Uint128::new(2)),
        (Direction::Ask, Price::new(53), Uint128::new(3)),
        (Direction::Ask, Price::new(54), Uint128::new(4)),
        (Direction::Ask, Price::new(55), Uint128::new(5)),
        (Direction::Ask, Price::new(56), Uint128::new(6)),
        (Direction::Ask, Price::new(57), Uint128::new(7)),
        (Direction::Ask, Price::new(58), Uint128::new(8)),
        (Direction::Ask, Price::new(59), Uint128::new(9)),
        (Direction::Ask, Price::new(60), Uint128::new(10)),
    ],
    None,
    Price::new(10),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(40), dex::LiquidityDepth {
                depth_base: Udec128_6::new(55),
                depth_quote: Udec128_6::new(2365),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(60), dex::LiquidityDepth {
                depth_base: Udec128_6::new(55),
                depth_quote: Udec128_6::new(3135),
            }),
        ]),
    },
    None;
    "multiple orders on both sides, bucket size 10"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(49), Uint128::new(1)),
        (Direction::Bid, Price::new(48), Uint128::new(2)),
        (Direction::Bid, Price::new(47), Uint128::new(3)),
        (Direction::Bid, Price::new(46), Uint128::new(4)),
        (Direction::Bid, Price::new(45), Uint128::new(5)),
        (Direction::Bid, Price::new(44), Uint128::new(6)),
        (Direction::Bid, Price::new(43), Uint128::new(7)),
        (Direction::Bid, Price::new(42), Uint128::new(8)),
        (Direction::Bid, Price::new(41), Uint128::new(9)),
        (Direction::Bid, Price::new(40), Uint128::new(10)),
        (Direction::Ask, Price::new(51), Uint128::new(1)),
        (Direction::Ask, Price::new(52), Uint128::new(2)),
        (Direction::Ask, Price::new(53), Uint128::new(3)),
        (Direction::Ask, Price::new(54), Uint128::new(4)),
        (Direction::Ask, Price::new(55), Uint128::new(5)),
        (Direction::Ask, Price::new(56), Uint128::new(6)),
        (Direction::Ask, Price::new(57), Uint128::new(7)),
        (Direction::Ask, Price::new(58), Uint128::new(8)),
        (Direction::Ask, Price::new(59), Uint128::new(9)),
        (Direction::Ask, Price::new(60), Uint128::new(10)),
    ],
    None,
    Price::new(1),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(49), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(49),
            }),
            (Price::new(48), dex::LiquidityDepth {
                depth_base: Udec128_6::new(2),
                depth_quote: Udec128_6::new(2 * 48),
            }),
            (Price::new(47), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3),
                depth_quote: Udec128_6::new(3 * 47),
            }),
            (Price::new(46), dex::LiquidityDepth {
                depth_base: Udec128_6::new(4),
                depth_quote: Udec128_6::new(4 * 46),
            }),
            (Price::new(45), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5),
                depth_quote: Udec128_6::new(5 * 45),
            }),
            (Price::new(44), dex::LiquidityDepth {
                depth_base: Udec128_6::new(6),
                depth_quote: Udec128_6::new(6 * 44),
            }),
            (Price::new(43), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7),
                depth_quote: Udec128_6::new(7 * 43),
            }),
            (Price::new(42), dex::LiquidityDepth {
                depth_base: Udec128_6::new(8),
                depth_quote: Udec128_6::new(8 * 42),
            }),
            (Price::new(41), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9),
                depth_quote: Udec128_6::new(9 * 41),
            }),
            (Price::new(40), dex::LiquidityDepth {
                depth_base: Udec128_6::new(10),
                depth_quote: Udec128_6::new(10 * 40),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(51), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(51),
            }),
            (Price::new(52), dex::LiquidityDepth {
                depth_base: Udec128_6::new(2),
                depth_quote: Udec128_6::new(2 * 52),
            }),
            (Price::new(53), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3),
                depth_quote: Udec128_6::new(3 * 53),
            }),
            (Price::new(54), dex::LiquidityDepth {
                depth_base: Udec128_6::new(4),
                depth_quote: Udec128_6::new(4 * 54),
            }),
            (Price::new(55), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5),
                depth_quote: Udec128_6::new(5 * 55),
            }),
            (Price::new(56), dex::LiquidityDepth {
                depth_base: Udec128_6::new(6),
                depth_quote: Udec128_6::new(6 * 56),
            }),
            (Price::new(57), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7),
                depth_quote: Udec128_6::new(7 * 57),
            }),
            (Price::new(58), dex::LiquidityDepth {
                depth_base: Udec128_6::new(8),
                depth_quote: Udec128_6::new(8 * 58),
            }),
            (Price::new(59), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9),
                depth_quote: Udec128_6::new(9 * 59),
            }),
            (Price::new(60), dex::LiquidityDepth {
                depth_base: Udec128_6::new(10),
                depth_quote: Udec128_6::new(10 * 60),
            }),
        ]),
    },
    None;
    "multiple orders on both sides, bucket size 1"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(50), Uint128::new(1)),
        (Direction::Bid, Price::new(49), Uint128::new(1)),
        (Direction::Bid, Price::new(48), Uint128::new(2)),
        (Direction::Bid, Price::new(47), Uint128::new(3)),
        (Direction::Bid, Price::new(46), Uint128::new(4)),
        (Direction::Bid, Price::new(45), Uint128::new(5)),
        (Direction::Bid, Price::new(44), Uint128::new(6)),
        (Direction::Bid, Price::new(43), Uint128::new(7)),
        (Direction::Bid, Price::new(42), Uint128::new(8)),
        (Direction::Bid, Price::new(41), Uint128::new(9)),
        (Direction::Bid, Price::new(40), Uint128::new(10)),
        (Direction::Ask, Price::new(51), Uint128::new(1)),
        (Direction::Ask, Price::new(52), Uint128::new(2)),
        (Direction::Ask, Price::new(53), Uint128::new(3)),
        (Direction::Ask, Price::new(54), Uint128::new(4)),
        (Direction::Ask, Price::new(55), Uint128::new(5)),
        (Direction::Ask, Price::new(56), Uint128::new(6)),
        (Direction::Ask, Price::new(57), Uint128::new(7)),
        (Direction::Ask, Price::new(58), Uint128::new(8)),
        (Direction::Ask, Price::new(59), Uint128::new(9)),
        (Direction::Ask, Price::new(60), Uint128::new(10)),
    ],
    None,
    Price::new(1),
    Some(3),
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(49), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(49),
            }),
            (Price::new(48), dex::LiquidityDepth {
                depth_base: Udec128_6::new(2),
                depth_quote: Udec128_6::new(2 * 48),
            }),
            (Price::new(47), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3),
                depth_quote: Udec128_6::new(3 * 47),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(51), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1),
                depth_quote: Udec128_6::new(51),
            }),
            (Price::new(52), dex::LiquidityDepth {
                depth_base: Udec128_6::new(2),
                depth_quote: Udec128_6::new(2 * 52),
            }),
            (Price::new(53), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3),
                depth_quote: Udec128_6::new(3 * 53),
            }),
        ]),
    },
    None;
    "multiple orders on both sides, one order per bucket, bucket size 1, limit 3"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(500), Uint128::new(1)),
        (Direction::Bid, Price::new(500), Uint128::new(1)),
        (Direction::Bid, Price::new(495), Uint128::new(1)),
        (Direction::Bid, Price::new(490), Uint128::new(2)),
        (Direction::Bid, Price::new(485), Uint128::new(3)),
        (Direction::Bid, Price::new(480), Uint128::new(4)),
        (Direction::Bid, Price::new(475), Uint128::new(5)),
        (Direction::Bid, Price::new(470), Uint128::new(6)),
        (Direction::Bid, Price::new(465), Uint128::new(7)),
        (Direction::Bid, Price::new(460), Uint128::new(8)),
        (Direction::Bid, Price::new(455), Uint128::new(9)),
        (Direction::Bid, Price::new(450), Uint128::new(10)),
        (Direction::Ask, Price::new(505), Uint128::new(1)),
        (Direction::Ask, Price::new(510), Uint128::new(2)),
        (Direction::Ask, Price::new(515), Uint128::new(3)),
        (Direction::Ask, Price::new(520), Uint128::new(4)),
        (Direction::Ask, Price::new(525), Uint128::new(5)),
        (Direction::Ask, Price::new(530), Uint128::new(6)),
        (Direction::Ask, Price::new(535), Uint128::new(7)),
        (Direction::Ask, Price::new(540), Uint128::new(8)),
        (Direction::Ask, Price::new(545), Uint128::new(9)),
        (Direction::Ask, Price::new(550), Uint128::new(10)),
    ],
    None,
    Price::new(10),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(490), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(495 + 2 * 490),
            }),
            (Price::new(480), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 485 + 4 * 480),
            }),
            (Price::new(470), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 475 + 6 * 470),
            }),
            (Price::new(460), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 465 + 8 * 460),
            }),
            (Price::new(450), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 455 + 10 * 450),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(510), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(505 + 2 * 510),
            }),
            (Price::new(520), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 515 + 4 * 520),
            }),
            (Price::new(530), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 525 + 6 * 530),
            }),
            (Price::new(540), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 535 + 8 * 540),
            }),
            (Price::new(550), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 545 + 10 * 550),
            }),
        ]),
    },
    None;
    "multiple orders on both sides, two orders per bucket, bucket size 20"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(500), Uint128::new(1)),
        (Direction::Bid, Price::new(500), Uint128::new(1)),
        (Direction::Bid, Price::new(495), Uint128::new(1)),
        (Direction::Bid, Price::new(490), Uint128::new(2)),
        (Direction::Bid, Price::new(485), Uint128::new(3)),
        (Direction::Bid, Price::new(480), Uint128::new(4)),
        (Direction::Bid, Price::new(475), Uint128::new(5)),
        (Direction::Bid, Price::new(470), Uint128::new(6)),
        (Direction::Bid, Price::new(465), Uint128::new(7)),
        (Direction::Bid, Price::new(460), Uint128::new(8)),
        (Direction::Bid, Price::new(455), Uint128::new(9)),
        (Direction::Bid, Price::new(450), Uint128::new(10)),
        (Direction::Ask, Price::new(505), Uint128::new(1)),
        (Direction::Ask, Price::new(510), Uint128::new(2)),
        (Direction::Ask, Price::new(515), Uint128::new(3)),
        (Direction::Ask, Price::new(520), Uint128::new(4)),
        (Direction::Ask, Price::new(525), Uint128::new(5)),
        (Direction::Ask, Price::new(530), Uint128::new(6)),
        (Direction::Ask, Price::new(535), Uint128::new(7)),
        (Direction::Ask, Price::new(540), Uint128::new(8)),
        (Direction::Ask, Price::new(545), Uint128::new(9)),
        (Direction::Ask, Price::new(550), Uint128::new(10)),
    ],
    Some(CancelOrderRequest::All),
    Price::new(10),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(490), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(495 + 2 * 490),
            }),
            (Price::new(480), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 485 + 4 * 480),
            }),
            (Price::new(470), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 475 + 6 * 470),
            }),
            (Price::new(460), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 465 + 8 * 460),
            }),
            (Price::new(450), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 455 + 10 * 450),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(510), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(505 + 2 * 510),
            }),
            (Price::new(520), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 515 + 4 * 520),
            }),
            (Price::new(530), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 525 + 6 * 530),
            }),
            (Price::new(540), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 535 + 8 * 540),
            }),
            (Price::new(550), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 545 + 10 * 550),
            }),
        ]),
    },
    Some(dex::LiquidityDepthResponse {
        bid_depth: None,
        ask_depth: None,
    });
    "multiple orders on both sides, two orders per bucket, bucket size 20, cancel all orders"
)]
#[test_case(
    vec![
        (Direction::Ask, Price::new(500), Uint128::new(1)),
        (Direction::Bid, Price::new(500), Uint128::new(1)),
        (Direction::Bid, Price::new(495), Uint128::new(1)),
        (Direction::Bid, Price::new(490), Uint128::new(2)),
        (Direction::Bid, Price::new(485), Uint128::new(3)),
        (Direction::Bid, Price::new(480), Uint128::new(4)),
        (Direction::Bid, Price::new(475), Uint128::new(5)),
        (Direction::Bid, Price::new(470), Uint128::new(6)),
        (Direction::Bid, Price::new(465), Uint128::new(7)),
        (Direction::Bid, Price::new(460), Uint128::new(8)),
        (Direction::Bid, Price::new(455), Uint128::new(9)),
        (Direction::Bid, Price::new(450), Uint128::new(10)),
        (Direction::Ask, Price::new(505), Uint128::new(1)),
        (Direction::Ask, Price::new(510), Uint128::new(2)),
        (Direction::Ask, Price::new(515), Uint128::new(3)),
        (Direction::Ask, Price::new(520), Uint128::new(4)),
        (Direction::Ask, Price::new(525), Uint128::new(5)),
        (Direction::Ask, Price::new(530), Uint128::new(6)),
        (Direction::Ask, Price::new(535), Uint128::new(7)),
        (Direction::Ask, Price::new(540), Uint128::new(8)),
        (Direction::Ask, Price::new(545), Uint128::new(9)),
        (Direction::Ask, Price::new(550), Uint128::new(10)),
    ],
    Some(CancelOrderRequest::Some(BTreeSet::from([
        OrderId::new(!7),
        OrderId::new(!10),
        OrderId::new(16),
    ]))),
    Price::new(10),
    None,
    dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(490), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(495 + 2 * 490),
            }),
            (Price::new(480), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 485 + 4 * 480),
            }),
            (Price::new(470), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 475 + 6 * 470),
            }),
            (Price::new(460), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 465 + 8 * 460),
            }),
            (Price::new(450), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 455 + 10 * 450),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(510), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(505 + 2 * 510),
            }),
            (Price::new(520), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 515 + 4 * 520),
            }),
            (Price::new(530), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 525 + 6 * 530),
            }),
            (Price::new(540), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 535 + 8 * 540),
            }),
            (Price::new(550), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 545 + 10 * 550),
            }),
        ]),
    },
    Some(dex::LiquidityDepthResponse {
        bid_depth: Some(vec![
            (Price::new(490), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(495 + 2 * 490),
            }),
            (Price::new(480), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3 + 4),
                depth_quote: Udec128_6::new(3 * 485 + 4 * 480),
            }),
            (Price::new(470), dex::LiquidityDepth {
                depth_base: Udec128_6::new(6),
                depth_quote: Udec128_6::new(6 * 470),
            }),
            (Price::new(460), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7),
                depth_quote: Udec128_6::new(7 * 465),
            }),
            (Price::new(450), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 455 + 10 * 450),
            }),
        ]),
        ask_depth: Some(vec![
            (Price::new(510), dex::LiquidityDepth {
                depth_base: Udec128_6::new(1 + 2),
                depth_quote: Udec128_6::new(505 + 2 * 510),
            }),
            (Price::new(520), dex::LiquidityDepth {
                depth_base: Udec128_6::new(3),
                depth_quote: Udec128_6::new(3 * 515),
            }),
            (Price::new(530), dex::LiquidityDepth {
                depth_base: Udec128_6::new(5 + 6),
                depth_quote: Udec128_6::new(5 * 525 + 6 * 530),
            }),
            (Price::new(540), dex::LiquidityDepth {
                depth_base: Udec128_6::new(7 + 8),
                depth_quote: Udec128_6::new(7 * 535 + 8 * 540),
            }),
            (Price::new(550), dex::LiquidityDepth {
                depth_base: Udec128_6::new(9 + 10),
                depth_quote: Udec128_6::new(9 * 545 + 10 * 550),
            }),
        ]),
    });
    "multiple orders on both sides, two orders per bucket, bucket size 20, cancel some orders"
)]
fn test_liquidity_depth_is_correctly_calculated_after_order_clearing_and_cancellation(
    limit_orders: Vec<(Direction, Price, Uint128)>, // direction, price, amount
    cancels: Option<CancelOrderRequest>,
    bucket_size: Price,
    limit: Option<u32>,
    expected_liquidity_depth_after_clearing: dex::LiquidityDepthResponse,
    expected_liquidity_depth_after_cancellation: Option<dex::LiquidityDepthResponse>,
) {
    // Setup test environment
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Configure the ETH-USDC pair with the bucket size we want to test.
    let mut params = suite
        .query_wasm_smart(contracts.dex, dex::QueryPairRequest {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
        })
        .should_succeed();
    params.bucket_sizes = btree_set! { NonZero::new_unchecked(bucket_size) };

    suite
        .execute(
            &mut accounts.owner,
            contracts.dex,
            &dex::ExecuteMsg::Owner(dex::OwnerMsg::BatchUpdatePairs(vec![PairUpdate {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                params,
            }])),
            Coins::new(),
        )
        .should_succeed();

    // Register oracle price sources for ETH and USDC
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
                eth::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    // Calculate required funds for market orders
    let mut required_eth = Uint128::ZERO;
    let mut required_usdc = Uint128::ZERO;

    // Calculate required funds for limit orders
    for (direction, price, amount) in &limit_orders {
        match direction {
            Direction::Bid => {
                // For bid orders, we need USDC (quote currency)
                let cost = amount.checked_mul_dec_ceil(*price).unwrap();
                required_usdc += cost;
            },
            Direction::Ask => {
                // For ask orders, we need ETH (base currency)
                required_eth += *amount;
            },
        }
    }

    // Create limit order requests
    let limit_order_requests: Vec<CreateOrderRequest> = limit_orders
        .into_iter()
        .map(|(direction, price, amount)| {
            CreateOrderRequest::new_limit(
                eth::DENOM.clone(),
                usdc::DENOM.clone(),
                direction,
                NonZero::new_unchecked(price),
                NonZero::new_unchecked(match direction {
                    Direction::Bid => amount.checked_mul_dec_ceil(price).unwrap(),
                    Direction::Ask => amount,
                }),
            )
        })
        .collect();

    // Build funds for the transaction
    let funds = match (required_eth > Uint128::ZERO, required_usdc > Uint128::ZERO) {
        (true, true) => coins! {
            eth::DENOM.clone() => required_eth,
            usdc::DENOM.clone() => required_usdc,
        },
        (true, false) => Coins::one(eth::DENOM.clone(), required_eth).unwrap(),
        (false, true) => Coins::one(usdc::DENOM.clone(), required_usdc).unwrap(),
        (false, false) => Coins::new(),
    };

    // Execute all orders in a single batch
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: limit_order_requests,
                cancels: None,
            },
            funds,
        )
        .should_succeed();

    // Query the liquidity depth
    suite
        .query_wasm_smart(contracts.dex, QueryLiquidityDepthRequest {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            bucket_size,
            limit,
        })
        .should_succeed_and_equal(expected_liquidity_depth_after_clearing.clone());

    // Cancel the orders
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![],
                cancels,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query the liquidity depth
    suite
        .query_wasm_smart(contracts.dex, QueryLiquidityDepthRequest {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            bucket_size,
            limit,
        })
        .should_succeed_and_equal(
            expected_liquidity_depth_after_cancellation
                .unwrap_or(expected_liquidity_depth_after_clearing),
        );
}

#[test]
fn decrease_liquidity_depths_minimal_failing_test() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    // Register oracle price sources for ETH and USDC
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

    // Submit new orders with user1
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::BatchUpdateOrders {
                creates: vec![
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
                creates: vec![CreateOrderRequest::new_limit(
                    eth::DENOM.clone(),
                    usdc::DENOM.clone(),
                    Direction::Ask,
                    NonZero::new_unchecked(
                        Price::from_str("85248.71").unwrap().checked_inv().unwrap(),
                    ),
                    NonZero::new_unchecked(Uint128::new(117304 * 2)),
                )],
                cancels: None,
            },
            coins! {
                eth::DENOM.clone() => 117304 * 2,
            },
        )
        .should_succeed();

    // Assert that orderbook is empty
    suite
        .query_wasm_smart(contracts.dex, dex::QueryOrdersByPairRequest {
            base_denom: eth::DENOM.clone(),
            quote_denom: usdc::DENOM.clone(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(BTreeMap::new());
}

#[test]
fn provide_liquidity_fails_when_minimum_output_is_not_met() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let expected_mint_amount = Uint128::new(100_000_000) - MINIMUM_LIQUIDITY;

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: Some(expected_mint_amount + Uint128::new(1)),
            },
            coins! {
                dango::DENOM.clone() => 100,
                usdc::DENOM.clone() => 100,
            },
        )
        .should_fail_with_error(format!(
            "LP mint amount is less than the minimum output: {} < {}",
            expected_mint_amount,
            expected_mint_amount + Uint128::new(1)
        ));
}

#[test]
fn withdraw_liquidity_fails_when_minimum_output_is_not_met() {
    let (mut suite, mut accounts, _, contracts, _) = setup_test_naive(Default::default());

    let lp_denom = Denom::try_from("dex/pool/dango/usdc").unwrap();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::ProvideLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: Some(Uint128::new(100_000_000) - MINIMUM_LIQUIDITY),
            },
            coins! {
                dango::DENOM.clone() => 100,
                usdc::DENOM.clone() => 100,
            },
        )
        .should_succeed();

    let lp_balance = suite
        .query_balance(&accounts.user1, lp_denom.clone())
        .unwrap();

    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::WithdrawLiquidity {
                base_denom: dango::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
                minimum_output: Some(
                    CoinPair::new(
                        Coin::new(dango::DENOM.clone(), Uint128::new(100)).unwrap(),
                        Coin::new(usdc::DENOM.clone(), Uint128::new(100)).unwrap(),
                    )
                    .unwrap(),
                ),
            },
            Coins::one(lp_denom.clone(), lp_balance).unwrap(),
        )
        .should_fail_with_error(format!(
            "withdrawn assets are less than the minimum output: {:?} < {:?}",
            CoinPair::try_from(coins! {
                usdc::DENOM.clone() => 99,
                dango::DENOM.clone() => 99,
            })
            .unwrap(),
            CoinPair::try_from(coins! {
                usdc::DENOM.clone() => 100,
                dango::DENOM.clone() => 100,
            })
            .unwrap(),
        ));
}
