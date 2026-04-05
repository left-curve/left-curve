use {
    dango_oracle::PRICE_SOURCES,
    dango_testing::setup_test_naive,
    dango_types::{
        constants::{dango, eth, usdc},
        dex::{
            self, Geometric, PairId, PairParams, PairUpdate, PassiveLiquidity, QueryReserveRequest,
        },
        oracle::PriceSource,
    },
    grug::{
        Addressable, BalanceChange, Bounded, Coin, CoinPair, Coins, Denom, Inner, LengthBounded,
        NonZero, NumberConst, QuerierExt, ResultExt, StdError, Udec128, Uint128, UniqueVec,
        btree_map, coins,
    },
    std::collections::{BTreeMap, BTreeSet},
    test_case::test_case,
};

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
                                    bucket_sizes: BTreeSet::new(),
                                    swap_fee_rate: Bounded::new_unchecked(swap_fee_rate),
                                    pool_type: pair_params.pool_type.clone(),
                                    min_order_size_quote: Uint128::ZERO,
                                    min_order_size_base: Uint128::ZERO,
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
                    minimum_output: None,
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
                route: LengthBounded::new_unchecked(UniqueVec::new_unchecked(route)),
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
        dango::DENOM.clone() => 1002007,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 1002007).unwrap(),
    Coin::new(usdc::DENOM.clone(), 2000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 1002007,
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
        dango::DENOM.clone() => 500752,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 500752).unwrap(),
    Coin::new(usdc::DENOM.clone(), 1334).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 500752,
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
        dango::DENOM.clone() => 333780,
    },
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => Udec128::new_permille(1),
    },
    Coin::new(dango::DENOM.clone(), 333780).unwrap(),
    Coin::new(usdc::DENOM.clone(), 1000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 333780,
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
    Coin::new(dango::DENOM.clone(), 1002007).unwrap(),
    Coin::new(usdc::DENOM.clone(), 2000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 1002007,
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
    Coin::new(dango::DENOM.clone(), 501761).unwrap(),
    Coin::new(eth::DENOM.clone(), 1000).unwrap(),
    btree_map! {
        (dango::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            dango::DENOM.clone() => 1000000 + 501761,
            usdc::DENOM.clone() => 1000000 - 333780,
        },
        (eth::DENOM.clone(), usdc::DENOM.clone()) => coins! {
            eth::DENOM.clone() => 1000000 - 250000,
            usdc::DENOM.clone() => 1000000 + 333780,
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
                    minimum_output: None,
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
                route: LengthBounded::new_unchecked(UniqueVec::new_unchecked(route)),
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
                minimum_output: None,
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

    // Ensure swap exact amount in fails
    suite
        .execute(
            &mut accounts.user1,
            contracts.dex,
            &dex::ExecuteMsg::SwapExactAmountIn {
                route: LengthBounded::new_unchecked(UniqueVec::new_unchecked(vec![PairId {
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
                route: LengthBounded::new_unchecked(UniqueVec::new_unchecked(vec![PairId {
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
