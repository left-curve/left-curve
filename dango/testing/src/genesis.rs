use {
    crate::{
        BridgeOp, TestOption,
        constants::{
            DEFAULT_GAS_LIMIT, MOCK_BLOCK_TIME, MOCK_CHAIN_ID, MOCK_GENESIS_TIMESTAMP, owner,
            user1, user2, user3, user4, user5, user6, user7, user8, user9,
        },
    },
    dango_genesis::{
        AccountOption, BankOption, DexOption, GatewayOption, GenesisOption, GenesisUser,
        GrugOption, HyperlaneOption, LendingOption, OracleOption, VestingOption,
    },
    dango_types::{
        auth::Key,
        bank::Metadata,
        constants::{PYTH_PRICE_SOURCES, btc, dango, eth, sol, usdc},
        dex::{CurveInvariant, PairParams, PairUpdate},
        gateway::{Remote, WithdrawalFee},
        lending::InterestRateModel,
        taxman,
    },
    grug::{
        Addressable, BlockInfo, Bounded, Coin, Denom, Duration, GENESIS_BLOCK_HASH,
        GENESIS_BLOCK_HEIGHT, HashExt, LengthBounded, NumberConst, Udec128, Uint128, btree_map,
        btree_set, coins,
    },
    hyperlane_testing::constants::{
        MOCK_HYPERLANE_LOCAL_DOMAIN, MOCK_HYPERLANE_VALIDATOR_ADDRESSES,
    },
    hyperlane_types::{
        constants::{arbitrum, base, ethereum, optimism, solana},
        isms::multisig::ValidatorSet,
    },
    pyth_types::constants::GUARDIAN_SETS,
    std::str::FromStr,
};

/// Describing a data that has a preset value for testing purposes.
pub trait Preset {
    fn preset_test() -> Self;
}

impl Preset for TestOption {
    fn preset_test() -> Self {
        TestOption {
            chain_id: MOCK_CHAIN_ID.to_string(),
            block_time: MOCK_BLOCK_TIME,
            default_gas_limit: DEFAULT_GAS_LIMIT,
            genesis_block: BlockInfo {
                hash: GENESIS_BLOCK_HASH,
                height: GENESIS_BLOCK_HEIGHT,
                timestamp: MOCK_GENESIS_TIMESTAMP,
            },
            // By default, give owner, user1, user2 each 100k USDC from Ethereum.
            bridge_ops: |accounts| {
                vec![
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.owner.address(),
                    },
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
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user2.address(),
                    },
                ]
            },
        }
    }
}

impl Preset for GenesisOption {
    fn preset_test() -> Self {
        GenesisOption {
            grug: Preset::preset_test(),
            account: Preset::preset_test(),
            bank: Preset::preset_test(),
            dex: Preset::preset_test(),
            gateway: Preset::preset_test(),
            hyperlane: Preset::preset_test(),
            lending: Preset::preset_test(),
            oracle: Preset::preset_test(),
            vesting: Preset::preset_test(),
        }
    }
}

impl Preset for GrugOption {
    fn preset_test() -> Self {
        GrugOption {
            owner_username: owner::USERNAME.clone(),
            fee_cfg: taxman::Config {
                fee_denom: usdc::DENOM.clone(),
                fee_rate: Udec128::ZERO, // Use zero gas price for testing.
            },
            max_orphan_age: Duration::from_weeks(1),
        }
    }
}

impl Preset for AccountOption {
    fn preset_test() -> Self {
        AccountOption {
            genesis_users: btree_map! {
                owner::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(owner::PUBLIC_KEY.into()),
                    key_hash: owner::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user1::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user1::PUBLIC_KEY.into()),
                    key_hash: user1::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user2::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user2::PUBLIC_KEY.into()),
                    key_hash: user2::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user3::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user3::PUBLIC_KEY.into()),
                    key_hash: user3::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user4::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user4::PUBLIC_KEY.into()),
                    key_hash: user4::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user5::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user5::PUBLIC_KEY.into()),
                    key_hash: user5::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user6::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user6::PUBLIC_KEY.into()),
                    key_hash: user6::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user7::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user7::PUBLIC_KEY.into()),
                    key_hash: user7::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user8::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user8::PUBLIC_KEY.into()),
                    key_hash: user8::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                user9::USERNAME.clone() => GenesisUser {
                    key: Key::Secp256k1(user9::PUBLIC_KEY.into()),
                    key_hash: user9::PUBLIC_KEY.hash256(),
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
            },
            minimum_deposit: coins! { usdc::DENOM.clone() => 10_000_000 },
        }
    }
}

impl Preset for BankOption {
    fn preset_test() -> Self {
        BankOption {
            metadatas: btree_map! {
                dango::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Dango".to_string()),
                    symbol: LengthBounded::new_unchecked("DGX".to_string()),
                    decimals: 6,
                    description: Some(LengthBounded::new_unchecked("Native token of Dango".to_string()))
                }
            },
        }
    }
}

impl Preset for DexOption {
    fn preset_test() -> Self {
        DexOption {
            pairs: vec![
                PairUpdate {
                    base_denom: dango::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                        curve_invariant: CurveInvariant::Xyk,
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    },
                },
                PairUpdate {
                    base_denom: btc::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                        curve_invariant: CurveInvariant::Xyk,
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    },
                },
                PairUpdate {
                    base_denom: eth::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/eth/usdc").unwrap(),
                        curve_invariant: CurveInvariant::Xyk,
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    },
                },
                PairUpdate {
                    base_denom: sol::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/sol/usdc").unwrap(),
                        curve_invariant: CurveInvariant::Xyk,
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                    },
                },
            ],
        }
    }
}

impl Preset for GatewayOption {
    fn preset_test() -> Self {
        GatewayOption {
            warp_routes: btree_set! {
                (usdc::SUBDENOM.clone(), Remote::Warp {
                    domain: arbitrum::DOMAIN,
                    contract: arbitrum::USDC_WARP,
                }),
                (usdc::SUBDENOM.clone(), Remote::Warp {
                    domain: base::DOMAIN,
                    contract: base::USDC_WARP,
                }),
                (usdc::SUBDENOM.clone(), Remote::Warp {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::USDC_WARP,
                }),
                (usdc::SUBDENOM.clone(), Remote::Warp {
                    domain: optimism::DOMAIN,
                    contract: optimism::USDC_WARP,
                }),
                (usdc::SUBDENOM.clone(), Remote::Warp {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                }),
                (eth::SUBDENOM.clone(), Remote::Warp {
                    domain: arbitrum::DOMAIN,
                    contract: arbitrum::WETH_WARP,
                }),
                (eth::SUBDENOM.clone(), Remote::Warp {
                    domain: base::DOMAIN,
                    contract: base::WETH_WARP,
                }),
                (eth::SUBDENOM.clone(), Remote::Warp {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::WETH_WARP,
                }),
                (eth::SUBDENOM.clone(), Remote::Warp {
                    domain: optimism::DOMAIN,
                    contract: optimism::WETH_WARP,
                }),
                (sol::SUBDENOM.clone(), Remote::Warp {
                    domain: solana::DOMAIN,
                    contract: solana::SOL_WARP,
                }),
            },
            withdrawal_fees: vec![
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: arbitrum::DOMAIN,
                        contract: arbitrum::USDC_WARP,
                    },
                    fee: Uint128::new(100_000),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: base::DOMAIN,
                        contract: base::USDC_WARP,
                    },
                    fee: Uint128::new(100_000),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::USDC_WARP,
                    },
                    fee: Uint128::new(1_000_000),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: optimism::DOMAIN,
                        contract: optimism::USDC_WARP,
                    },
                    fee: Uint128::new(100_000),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: solana::DOMAIN,
                        contract: solana::USDC_WARP,
                    },
                    fee: Uint128::new(10_000),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: arbitrum::DOMAIN,
                        contract: arbitrum::WETH_WARP,
                    },
                    fee: Uint128::new(50_000_000_000_000),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: base::DOMAIN,
                        contract: base::WETH_WARP,
                    },
                    fee: Uint128::new(50_000_000_000_000),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::WETH_WARP,
                    },
                    fee: Uint128::new(500_000_000_000_000),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: optimism::DOMAIN,
                        contract: optimism::WETH_WARP,
                    },
                    fee: Uint128::new(50_000_000_000_000),
                },
                WithdrawalFee {
                    denom: sol::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: solana::DOMAIN,
                        contract: solana::SOL_WARP,
                    },
                    fee: Uint128::new(66_667), // ~$0.01, assume SOL is $150
                },
            ],
            rate_limits: btree_map! {
                usdc::DENOM.clone() => Bounded::new_unchecked(Udec128::new_percent(10)),
                eth::DENOM.clone()  => Bounded::new_unchecked(Udec128::new_percent(10)),
                sol::DENOM.clone()  => Bounded::new_unchecked(Udec128::new_percent(10)),
            },
            rate_limit_refresh_period: Duration::from_days(1),
        }
    }
}

impl Preset for HyperlaneOption {
    fn preset_test() -> Self {
        // We use the same mock validator set for all remote domains.
        let mock_validator_set = ValidatorSet {
            threshold: 2,
            validators: MOCK_HYPERLANE_VALIDATOR_ADDRESSES.into_iter().collect(),
        };

        HyperlaneOption {
            local_domain: MOCK_HYPERLANE_LOCAL_DOMAIN,
            ism_validator_sets: btree_map! {
                arbitrum::DOMAIN => mock_validator_set.clone(),
                base::DOMAIN     => mock_validator_set.clone(),
                ethereum::DOMAIN => mock_validator_set.clone(),
                optimism::DOMAIN => mock_validator_set.clone(),
                solana::DOMAIN   => mock_validator_set,
            },
            va_announce_fee_per_byte: Coin {
                denom: usdc::DENOM.clone(),
                amount: Uint128::new(100),
            },
        }
    }
}

impl Preset for LendingOption {
    fn preset_test() -> Self {
        LendingOption {
            markets: btree_map! {
                usdc::DENOM.clone() => InterestRateModel::mock(),
            },
        }
    }
}

impl Preset for OracleOption {
    fn preset_test() -> Self {
        OracleOption {
            pyth_price_sources: PYTH_PRICE_SOURCES.clone(),
            wormhole_guardian_sets: GUARDIAN_SETS.clone(),
        }
    }
}

impl Preset for VestingOption {
    fn preset_test() -> Self {
        VestingOption {
            unlocking_cliff: Duration::from_weeks(4 * 9), // ~9 months
            unlocking_period: Duration::from_weeks(4 * 27), // ~27 months
        }
    }
}
