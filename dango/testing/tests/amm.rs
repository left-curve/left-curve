use {
    dango_testing::setup_test,
    dango_types::{
        amm::{
            self, ExecuteMsg, FeeRate, Pool, PoolParams, QueryPoolRequest, QueryPoolsRequest,
            XykParams, XykPool, MINIMUM_LIQUIDITY,
        },
        config::DANGO_DENOM,
    },
    grug::{
        btree_map, Coin, CoinPair, Coins, Denom, Message, NonEmpty, ResultExt, Udec128, Uint128,
        UniqueVec,
    },
    std::{str::FromStr, sync::LazyLock},
};

static ATOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uatom").unwrap());
static OSMO: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uosmo").unwrap());
static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());
static LP_1: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("amm/pool/1").unwrap());
static LP_2: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("amm/pool/2").unwrap());

#[test]
fn amm() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    // ----------------------------- Pool creation -----------------------------

    // Create two pools with ATOM-OSMO and ATOM-USDC liquidity, respectively.
    suite
        .send_messages(
            &mut accounts.user1,
            NonEmpty::new_unchecked(vec![
                // pool 1: ATOM-OSMO
                Message::execute(
                    contracts.amm,
                    &amm::ExecuteMsg::CreatePool(PoolParams::Xyk(XykParams {
                        liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                    })),
                    Coins::new_unchecked(btree_map! {
                        ATOM.clone() => Uint128::new(657_761_324_779),
                        OSMO.clone() => Uint128::new(5_886_161_498_040),
                        // pool creation fee
                        USDC.clone() => Uint128::new(10_000_000),
                    }),
                )
                .unwrap(),
                // pool 2: ATOM-USDC
                Message::execute(
                    contracts.amm,
                    &amm::ExecuteMsg::CreatePool(PoolParams::Xyk(XykParams {
                        liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                    })),
                    Coins::new_unchecked(btree_map! {
                        ATOM.clone() => Uint128::new(224_078_907_873),
                        // liquidity + pool creation fee
                        USDC.clone() => Uint128::new(173_573_581_955),
                    }),
                )
                .unwrap(),
            ]),
        )
        .should_succeed();

    // Check the pools.
    suite
        .query_wasm_smart(contracts.amm, QueryPoolsRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map! {
            1 => Pool::Xyk(XykPool {
                params:XykParams {
                    liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                },
                liquidity: CoinPair::new_unchecked(
                    Coin {
                        denom: ATOM.clone(),
                        amount: Uint128::new(657_761_324_779),
                    },
                    Coin {
                        denom: OSMO.clone(),
                        amount: Uint128::new(5_886_161_498_040),
                    },
                ),
                // floor(sqrt(657,761,324,779 * 5,886,161,498,040))
                shares: Uint128::new(1_967_660_891_722),
            }),
            2 => Pool::Xyk(XykPool {
                params:XykParams {
                    liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                },
                liquidity: CoinPair::new_unchecked(
                    Coin {
                        denom: ATOM.clone(),
                        amount: Uint128::new(224_078_907_873),
                    },
                    Coin {
                        denom: USDC.clone(),
                        // Note that pool creation fee is subtracted.
                        amount: Uint128::new(173_563_581_955),
                    },
                ),
                // floor(sqrt(224,078,907,873 * 173,563,581,955))
                shares: Uint128::new(197_210_389_916),
            }),
        });

    // Check the AMM contract's balance.
    suite
        .query_balances(&contracts.amm)
        .should_succeed_and_equal(Coins::new_unchecked(btree_map! {
            // 657,761,324,779 + 224,078,907,873
            ATOM.clone() => Uint128::new(881_840_232_652),
            OSMO.clone() => Uint128::new(5_886_161_498_040),
            USDC.clone() => Uint128::new(173_563_581_955),
            LP_1.clone() => MINIMUM_LIQUIDITY,
            LP_2.clone() => MINIMUM_LIQUIDITY,
        }));

    // Check the pool creator's LP token balances.
    suite
        .query_balances(&accounts.user1)
        .should_succeed_and_equal(Coins::new_unchecked(btree_map! {
            DANGO_DENOM.clone() => Uint128::new(100_000_000_000_000),
            // 100,000,000,000,000 - 657,761,324,779 - 224,078,907,873
            ATOM.clone() => Uint128::new(99_118_159_767_348),
            // 100,000,000,000,000 - 5,886,161,498,040
            OSMO.clone() => Uint128::new(94_113_838_501_960),
            // 100,000,000,000,000 - 173,573,581,955 - 10,000,000
            USDC.clone() => Uint128::new(99_826_416_418_045),
            // 1,967,660,891,722 - MINIMUM_LIQUIDITY
            LP_1.clone() => Uint128::new(1_967_660_890_722),
            // 197,210,389,916 - MINIMUM_LIQUIDITY
            LP_2.clone() => Uint128::new(197_210_388_916),
        }));

    // Check the taxman has received the pool creation fees.
    suite
        .query_balance(&contracts.taxman, USDC.clone())
        .should_succeed_and_equal(Uint128::new(20_000_000));

    // -------------------------- Liquidity provision --------------------------

    // Provide two sided liquidity to pool 1 (ATOM-OSMO).
    //
    // shares_before = 1,967,660,891,722
    // atom_before = 657,761,324,779
    // osmo_before = 5,886,161,498,040
    // atom_add = 6,577,613
    // osmo_add = 58,861,614
    //
    // atom_after = atom_before + atom_add
    // = 657,761,324,779 + 6,577,613
    // = 657,767,902,392
    //
    // osmo_after = osmo_before + osmo_add
    // = 5,886,161,498,040 + 58,861,614
    // = 5,886,220,359,654
    //
    // shares_after = sqrt((shares_before ^ 2) * atom_after * osmo_after / atom_before / osmo_before)
    // = sqrt((1,967,660,891,722 ^ 2) * 657,767,902,392 * 5,886,220,359,654 / 657,761,324,779 / 5,886,161,498,040)
    // = 1,967,680,568,330
    //
    // shares_to_mint = shares_after - shares_before
    // = 1,967,680,568,330 - 1,967,660,891,722
    // = 19,676,608
    //
    // ----------
    //
    // Provide one-sided liquidity to pool 2 (ATOM-USDC).
    //
    // shares_before = 197,210,389,916
    // atom_before = 224,078,907,873
    // usdc_before = 173,563,581,955
    // atom_add = 100,000,000
    // usdc_add = 0
    //
    // atom_after = atom_before + atom_add
    // = 224,078,907,873 + 100,000,000
    // = 224,178,907,873
    //
    // usdc_after = usdc_before + usdc_add
    // = 173,563,581,955 + 0
    // = 173,563,581,955
    //
    // shares_after = sqrt((shares_before ^ 2) * atom_after * usdc_after / atom_before / usdc_before)
    // = sqrt((197,210,389,916 ^ 2) * 224,178,907,873 * 173,563,581,955 / 224,078,907,873 / 173,563,581,955)
    // = 197,254,389,682
    //
    // shares_to_mint = shares_after - shares_before
    // = 197,254,389,682 - 197,210,389,916
    // = 44,999,766
    suite
        .send_messages(
            &mut accounts.user1,
            NonEmpty::new_unchecked(vec![
                Message::execute(
                    contracts.amm,
                    &ExecuteMsg::ProvideLiquidity {
                        pool_id: 1,
                        minimum_output: None,
                    },
                    Coins::new_unchecked(btree_map! {
                        ATOM.clone() => Uint128::new(6_577_613),
                        OSMO.clone() => Uint128::new(58_861_614),
                    }),
                )
                .unwrap(),
                Message::execute(
                    contracts.amm,
                    &ExecuteMsg::ProvideLiquidity {
                        pool_id: 2,
                        minimum_output: None,
                    },
                    Coins::new_unchecked(btree_map! {
                        ATOM.clone() => Uint128::new(100_000_000),
                    }),
                )
                .unwrap(),
            ]),
        )
        .should_succeed();

    // Check pool states should have been updated.
    suite
        .query_wasm_smart(contracts.amm, QueryPoolsRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map! {
            1 => Pool::Xyk(XykPool {
                params:XykParams {
                    liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                },
                liquidity: CoinPair::new_unchecked(
                    Coin {
                        denom: ATOM.clone(),
                        amount: Uint128::new(657_767_902_392),
                    },
                    Coin {
                        denom: OSMO.clone(),
                        amount: Uint128::new(5_886_220_359_654),
                    },
                ),
                shares: Uint128::new(1_967_680_568_330),
            }),
            2 => Pool::Xyk(XykPool {
                params:XykParams {
                    liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                },
                liquidity: CoinPair::new_unchecked(
                    Coin {
                        denom: ATOM.clone(),
                        amount: Uint128::new(224_178_907_873),
                    },
                    Coin {
                        denom: USDC.clone(),
                        // Note that pool creation fee is subtracted.
                        amount: Uint128::new(173_563_581_955),
                    },
                ),
                shares: Uint128::new(197_254_389_682),
            }),
        });

    // Check the pool creator's token balances.
    suite
        .query_balances(&accounts.user1)
        .should_succeed_and_equal(Coins::new_unchecked(btree_map! {
            DANGO_DENOM.clone() => Uint128::new(100_000_000_000_000),
            // 99_118_159_767_348 - 6_577_613 - 100_000_000
            ATOM.clone() => Uint128::new(99_118_053_189_735),
            // 94_113_838_501_960 - 58_861_614
            OSMO.clone() => Uint128::new(94_113_779_640_346),
            // unchanged
            USDC.clone() => Uint128::new(99_826_416_418_045),
            // 1,967,660,890,722 + 19,676,608 = 1,967,680,567,330
            LP_1.clone() => Uint128::new(1_967_680_567_330),
            // 197,210,388,916 + 44,999,766 = 197,254,388,682
            LP_2.clone() => Uint128::new(197_254_388_682),
        }));

    // --------------------------------- Swap ----------------------------------

    // Swap USDC for OSMO.
    suite
        .execute(
            &mut accounts.owner,
            contracts.amm,
            &ExecuteMsg::Swap {
                route: UniqueVec::new_unchecked(vec![2, 1]),
                minimum_output: None,
            },
            Coin::new(USDC.clone(), Uint128::new(100_000_000)).unwrap(),
        )
        .should_succeed();

    // Check the trader has received the correct amount of OSMO.
    //
    // Pool 2: USDC --> ATOM
    // Pool balances: 224,178,907,873 uatom + 173,563,581,955 uusdc
    // output_before_fee = pool_atom - pool_atom * pool_usdc / (pool_usdc + input_usdc)
    // = 224,178,907,873 - 224,178,907,873 * 173,563,581,955 / (173,563,581,955 + 100,000,000)
    // = 224,178,907,873 - 224,049,819,836
    // = 129,088,037
    // liquidity_fee = 129,088,037 * 20 / 10,000 = 258,177 (Note: ceil)
    // output = 129,088,037 - 258,177 = 128,829,860
    //
    // Pool 1: ATOM --> OSMO
    // Pool balances: 657,767,902,392 uatom + 5,886,220,359,654 uosmo
    // output_before_fee = pool_osmo - pool_osmo * pool_atom / (pool_atom + input_atom)
    // = 5,886,220,359,654 - 5,886,220,359,654 * 657,767,902,392 / (657,767,902,392 + 128,829,860)
    // = 5,886,220,359,654 - 5,885,067,715,313
    // = 1,152,644,341
    // liquidity_fee = 1,152,644,341 * 20 / 10,000 = 2,305,289
    // output = 1,152,644,341 - 2,305,289 = 1,150,339,052
    //
    // protocol_fee = 1,150,339,052 * 10 / 10,000 = 1,150,340 (Note: ceil)
    // output = 1,150,339,052 - 1,150,340 = 1,149,188,712
    suite
        .query_balance(&accounts.owner, OSMO.clone())
        .should_succeed_and_equal(Uint128::new(1_149_188_712));

    // Check that taxman has received the protocol fee.
    suite
        .query_balance(&contracts.taxman, OSMO.clone())
        .should_succeed_and_equal(Uint128::new(1_150_340));

    // The pool states should have been updated.
    //
    // Pool 1
    // uatom = 657,767,902,392 + 128,829,860 = 657,896,732,252
    // uosmo = 5,886,220,359,654 - 1,150,339,052 = 5,885,070,020,602
    //
    // Pool 2
    // uatom = 224,178,907,873 - 128,829,860 = 224,050,078,013
    // uusdc = 173,563,581,955 + 100,000,000 = 173,663,581,955
    suite
        .query_wasm_smart(contracts.amm, QueryPoolsRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(btree_map! {
            1 => Pool::Xyk(XykPool {
                params:XykParams {
                    liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                },
                liquidity: CoinPair::new_unchecked(
                    Coin {
                        denom: ATOM.clone(),
                        amount: Uint128::new(657_896_732_252),
                    },
                    Coin {
                        denom: OSMO.clone(),
                        amount: Uint128::new(5_885_070_020_602),
                    },
                ),
                shares: Uint128::new(1_967_680_568_330),
            }),
            2 => Pool::Xyk(XykPool {
                params:XykParams {
                    liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
                },
                liquidity: CoinPair::new_unchecked(
                    Coin {
                        denom: ATOM.clone(),
                        amount: Uint128::new(224_050_078_013),
                    },
                    Coin {
                        denom: USDC.clone(),
                        // Note that pool creation fee is subtracted.
                        amount: Uint128::new(173_663_581_955),
                    },
                ),
                shares: Uint128::new(197_254_389_682),
            }),
        });

    // ------------------------- Liquidity withdrawal --------------------------

    // The liquidity provider withdraws around 1/3 of their ATOM-OSMO liquidity.
    suite
        .execute(
            &mut accounts.user1,
            contracts.amm,
            &ExecuteMsg::WithdrawLiquidity { pool_id: 1 },
            Coin::new(LP_1.clone(), Uint128::new(655_886_963_574)).unwrap(),
        )
        .should_succeed();

    // Check LPer has received the correct amount of ATOM and OSMO.
    //
    // uatom_received
    // = 657,896,732,252 * 655,886,963,574 / 1,967,680,568,330
    // = 219,296,717,672
    //
    // uosmo received
    // = 5,885,070,020,602 * 655,886,963,574 / 1,967,680,568,330
    // = 1,961,670,389,167
    suite
        .query_balances(&accounts.user1)
        .should_succeed_and_equal(Coins::new_unchecked(btree_map! {
            DANGO_DENOM.clone() => Uint128::new(100_000_000_000_000),
            // 99,118,053,189,735 + 219,296,717,672
            ATOM.clone() => Uint128::new(99_337_349_907_407),
            // 94,113,779,640,346 + 1,961,670,389,167
            OSMO.clone() => Uint128::new(96_075_450_029_513),
            // unchanged
            USDC.clone() => Uint128::new(99_826_416_418_045),
            // 1,967,680,567,330 - 655_886_963_574
            LP_1.clone() => Uint128::new(1_311_793_603_756),
            // unchanged
            LP_2.clone() => Uint128::new(197_254_388_682),
        }));

    // Check pool states.
    suite
        .query_wasm_smart(contracts.amm, QueryPoolRequest { pool_id: 1 })
        .should_succeed_and_equal(Pool::Xyk(XykPool {
            params: XykParams {
                liquidity_fee_rate: FeeRate::new_unchecked(Udec128::new_bps(20)),
            },
            liquidity: CoinPair::new_unchecked(
                Coin {
                    denom: ATOM.clone(),
                    // 657,896,732,252 - 219,296,717,672
                    amount: Uint128::new(438_600_014_580),
                },
                Coin {
                    denom: OSMO.clone(),
                    // 5,885,070,020,602 - 1,961,670,389,167
                    amount: Uint128::new(3_923_399_631_435),
                },
            ),
            // 1,967,680,568,330 - 655_886_963_574
            shares: Uint128::new(1_311_793_604_756),
        }));
}
