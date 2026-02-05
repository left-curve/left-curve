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
        GrugOption, HyperlaneOption, OracleOption, VestingOption,
    },
    dango_types::{
        account_factory::NewUserSalt,
        auth::Key,
        bank::Metadata,
        constants::{
            PYTH_PRICE_SOURCES, atom, bch, bnb, btc, btc_usdc, dango, doge, eth, eth_usdc, ltc,
            sol, sol_usdc, usdc, xrp,
        },
        dex::{PairParams, PairUpdate, PassiveLiquidity, Xyk},
        gateway::{Origin, Remote, WithdrawalFee},
        taxman::{self, CommissionRebund, ReferralConfig},
    },
    grug::{
        Addressable, Binary, BlockInfo, Bounded, Coin, Coins, Denom, Duration, GENESIS_BLOCK_HASH,
        GENESIS_BLOCK_HEIGHT, HashExt, LengthBounded, NumberConst, Op, Timestamp, Udec128, Uint128,
        btree_map, btree_set,
    },
    hyperlane_testing::constants::{
        MOCK_HYPERLANE_LOCAL_DOMAIN, MOCK_HYPERLANE_VALIDATOR_ADDRESSES,
    },
    hyperlane_types::{
        constants::{arbitrum, base, ethereum, ethereum_testnet, optimism, solana},
        isms::multisig::ValidatorSet,
    },
    pyth_types::constants::LAZER_TRUSTED_SIGNER,
    std::{collections::BTreeSet, str::FromStr},
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
            mocked_clickhouse: false,
            // By default, give the owner and each user 100k USDC from Ethereum.
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
                            contract: ethereum::ETH_WARP,
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
                            contract: ethereum::ETH_WARP,
                        },
                        amount: Uint128::new(20_000_000_000_000_000_000), // 20 ETH
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
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::ETH_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user2.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user3.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user4.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user5.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user6.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user7.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user8.address(),
                    },
                    BridgeOp {
                        remote: Remote::Warp {
                            domain: ethereum::DOMAIN,
                            contract: ethereum::USDC_WARP,
                        },
                        amount: Uint128::new(100_000_000_000),
                        recipient: accounts.user9.address(),
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
            oracle: Preset::preset_test(),
            vesting: Preset::preset_test(),
        }
    }
}

impl Preset for GrugOption {
    fn preset_test() -> Self {
        GrugOption {
            owner_index: 0,
            fee_cfg: taxman::Config {
                fee_denom: usdc::DENOM.clone(),
                fee_rate: Udec128::ZERO, // Use zero gas price for testing.
                referral: ReferralConfig {
                    volume_to_be_referrer: Uint128::new(0), // No volume requirement for testing.
                    commission_rebound_default: CommissionRebund::new(
                        Udec128::checked_from_ratio(10, 100).unwrap(),
                    )
                    .unwrap(),
                    commission_rebound_by_volume: btree_map!(
                        Uint128::new(100_000) => CommissionRebund::new(Udec128::new_percent(20)).unwrap(),
                        Uint128::new(1_000_000) => CommissionRebund::new(Udec128::new_percent(30)).unwrap(),
                        Uint128::new(10_000_000) => CommissionRebund::new(Udec128::new_percent(40)).unwrap(),
                    ),
                },
            },
            max_orphan_age: Duration::from_weeks(1),
        }
    }
}

impl Preset for AccountOption {
    fn preset_test() -> Self {
        AccountOption {
            genesis_users: vec![
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(owner::PUBLIC_KEY.into()),
                        key_hash: owner::PUBLIC_KEY.hash256(),
                        seed: 0,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user1::PUBLIC_KEY.into()),
                        key_hash: user1::PUBLIC_KEY.hash256(),
                        seed: 1,
                    },
                    dango_balance: Uint128::new(1_000_000_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user2::PUBLIC_KEY.into()),
                        key_hash: user2::PUBLIC_KEY.hash256(),
                        seed: 2,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user3::PUBLIC_KEY.into()),
                        key_hash: user3::PUBLIC_KEY.hash256(),
                        seed: 3,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user4::PUBLIC_KEY.into()),
                        key_hash: user4::PUBLIC_KEY.hash256(),
                        seed: 4,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user5::PUBLIC_KEY.into()),
                        key_hash: user5::PUBLIC_KEY.hash256(),
                        seed: 5,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user6::PUBLIC_KEY.into()),
                        key_hash: user6::PUBLIC_KEY.hash256(),
                        seed: 6,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user7::PUBLIC_KEY.into()),
                        key_hash: user7::PUBLIC_KEY.hash256(),
                        seed: 7,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user8::PUBLIC_KEY.into()),
                        key_hash: user8::PUBLIC_KEY.hash256(),
                        seed: 8,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
                GenesisUser {
                    salt: NewUserSalt {
                        key: Key::Secp256k1(user9::PUBLIC_KEY.into()),
                        key_hash: user9::PUBLIC_KEY.hash256(),
                        seed: 9,
                    },
                    dango_balance: Uint128::new(100_000_000_000_000),
                },
            ],
            minimum_deposit: Coins::new(),
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
                    decimals: dango::DECIMAL,
                    description: Some(LengthBounded::new_unchecked("Native token of Dango".to_string())),
                },
                atom::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Atom".to_string()),
                    symbol: LengthBounded::new_unchecked("ATOM".to_string()),
                    decimals: atom::DECIMAL,
                    description: None,
                },
                bch::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Bitcoin Cash".to_string()),
                    symbol: LengthBounded::new_unchecked("BCH".to_string()),
                    decimals: bch::DECIMAL,
                    description: None,
                },
                bnb::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Binance Coin".to_string()),
                    symbol: LengthBounded::new_unchecked("BNB".to_string()),
                    decimals: bnb::DECIMAL,
                    description: None,
                },
                btc::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Bitcoin".to_string()),
                    symbol: LengthBounded::new_unchecked("BTC".to_string()),
                    decimals: btc::DECIMAL,
                    description: None,
                },
                doge::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Dogecoin".to_string()),
                    symbol: LengthBounded::new_unchecked("DOGE".to_string()),
                    decimals: doge::DECIMAL,
                    description: None,
                },
                eth::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Ether".to_string()),
                    symbol: LengthBounded::new_unchecked("ETH".to_string()),
                    decimals: eth::DECIMAL,
                    description: None,
                },
                ltc::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Litecoin".to_string()),
                    symbol: LengthBounded::new_unchecked("LTC".to_string()),
                    decimals: ltc::DECIMAL,
                    description: None,
                },
                sol::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("Solana".to_string()),
                    symbol: LengthBounded::new_unchecked("SOL".to_string()),
                    decimals: sol::DECIMAL,
                    description: None,
                },
                usdc::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("USD Coin".to_string()),
                    symbol: LengthBounded::new_unchecked("USDC".to_string()),
                    decimals: usdc::DECIMAL,
                    description: None,
                },
                xrp::DENOM.clone() => Metadata {
                    name: LengthBounded::new_unchecked("XRP".to_string()),
                    symbol: LengthBounded::new_unchecked("XRP".to_string()),
                    decimals: xrp::DECIMAL,
                    description: None,
                },
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
                        pool_type: PassiveLiquidity::Xyk(Xyk {
                            spacing: Udec128::ONE,
                            reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                            limit: 30,
                        }),
                        bucket_sizes: BTreeSet::new(), /* TODO: determine appropriate price buckets based on expected dango token price */
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::new(50), /* TODO: for mainnet, a minimum of $10 is sensible */
                        min_order_size_base: Uint128::new(2),
                    },
                },
                PairUpdate {
                    base_denom: btc::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Xyk(Xyk {
                            spacing: Udec128::ONE,
                            reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                            limit: 30,
                        }),
                        bucket_sizes: btree_set! {
                            btc_usdc::ONE_HUNDREDTH,
                            btc_usdc::ONE_TENTH,
                            btc_usdc::ONE,
                            btc_usdc::TEN,
                            btc_usdc::FIFTY,
                            btc_usdc::ONE_HUNDRED,
                        },
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
                    },
                },
                PairUpdate {
                    base_denom: eth::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/eth/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Xyk(Xyk {
                            spacing: Udec128::ONE,
                            reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                            limit: 30,
                        }),
                        bucket_sizes: btree_set! {
                            eth_usdc::ONE_HUNDREDTH,
                            eth_usdc::ONE_TENTH,
                            eth_usdc::ONE,
                            eth_usdc::TEN,
                            eth_usdc::FIFTY,
                            eth_usdc::ONE_HUNDRED,
                        },
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
                    },
                },
                PairUpdate {
                    base_denom: sol::DENOM.clone(),
                    quote_denom: usdc::DENOM.clone(),
                    params: PairParams {
                        lp_denom: Denom::from_str("dex/pool/sol/usdc").unwrap(),
                        pool_type: PassiveLiquidity::Xyk(Xyk {
                            spacing: Udec128::ONE,
                            reserve_ratio: Bounded::new_unchecked(Udec128::ZERO),
                            limit: 30,
                        }),
                        bucket_sizes: btree_set! {
                            sol_usdc::ONE_HUNDREDTH,
                            sol_usdc::ONE_TENTH,
                            sol_usdc::ONE,
                            sol_usdc::TEN,
                        },
                        swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                        min_order_size_quote: Uint128::ZERO,
                        min_order_size_base: Uint128::ZERO,
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
                (Origin::Remote(usdc::SUBDENOM.clone()), Remote::Warp {
                    domain: arbitrum::DOMAIN,
                    contract: arbitrum::USDC_WARP,
                }),
                (Origin::Remote(usdc::SUBDENOM.clone()), Remote::Warp {
                    domain: base::DOMAIN,
                    contract: base::USDC_WARP,
                }),
                (Origin::Remote(usdc::SUBDENOM.clone()), Remote::Warp {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::USDC_WARP,
                }),
                (Origin::Remote(usdc::SUBDENOM.clone()), Remote::Warp {
                    domain: optimism::DOMAIN,
                    contract: optimism::USDC_WARP,
                }),
                (Origin::Remote(usdc::SUBDENOM.clone()), Remote::Warp {
                    domain: solana::DOMAIN,
                    contract: solana::USDC_WARP,
                }),
                (Origin::Remote(eth::SUBDENOM.clone()), Remote::Warp {
                    domain: arbitrum::DOMAIN,
                    contract: arbitrum::ETH_WARP,
                }),
                (Origin::Remote(eth::SUBDENOM.clone()), Remote::Warp {
                    domain: base::DOMAIN,
                    contract: base::ETH_WARP,
                }),
                (Origin::Remote(eth::SUBDENOM.clone()), Remote::Warp {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::ETH_WARP,
                }),
                (Origin::Remote(eth::SUBDENOM.clone()), Remote::Warp {
                    domain: optimism::DOMAIN,
                    contract: optimism::ETH_WARP,
                }),
                (Origin::Remote(sol::SUBDENOM.clone()), Remote::Warp {
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
                    fee: Op::Insert(Uint128::new(100_000)),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: base::DOMAIN,
                        contract: base::USDC_WARP,
                    },
                    fee: Op::Insert(Uint128::new(100_000)),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::USDC_WARP,
                    },
                    fee: Op::Insert(Uint128::new(1_000_000)),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: optimism::DOMAIN,
                        contract: optimism::USDC_WARP,
                    },
                    fee: Op::Insert(Uint128::new(100_000)),
                },
                WithdrawalFee {
                    denom: usdc::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: solana::DOMAIN,
                        contract: solana::USDC_WARP,
                    },
                    fee: Op::Insert(Uint128::new(10_000)),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: arbitrum::DOMAIN,
                        contract: arbitrum::ETH_WARP,
                    },
                    fee: Op::Insert(Uint128::new(50_000_000_000_000)),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: base::DOMAIN,
                        contract: base::ETH_WARP,
                    },
                    fee: Op::Insert(Uint128::new(50_000_000_000_000)),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: ethereum::ETH_WARP,
                    },
                    fee: Op::Insert(Uint128::new(500_000_000_000_000)),
                },
                WithdrawalFee {
                    denom: eth::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: optimism::DOMAIN,
                        contract: optimism::ETH_WARP,
                    },
                    fee: Op::Insert(Uint128::new(50_000_000_000_000)),
                },
                WithdrawalFee {
                    denom: sol::DENOM.clone(),
                    remote: Remote::Warp {
                        domain: solana::DOMAIN,
                        contract: solana::SOL_WARP,
                    },
                    fee: Op::Insert(Uint128::new(66_667)), // ~$0.01, assume SOL is $150
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
                arbitrum::DOMAIN         => mock_validator_set.clone(),
                base::DOMAIN             => mock_validator_set.clone(),
                ethereum::DOMAIN         => mock_validator_set.clone(),
                ethereum_testnet::DOMAIN => mock_validator_set.clone(),
                optimism::DOMAIN         => mock_validator_set.clone(),
                solana::DOMAIN           => mock_validator_set,
            },
            va_announce_fee_per_byte: Coin {
                denom: usdc::DENOM.clone(),
                amount: Uint128::new(100),
            },
        }
    }
}

impl Preset for OracleOption {
    fn preset_test() -> Self {
        OracleOption {
            pyth_price_sources: PYTH_PRICE_SOURCES.clone(),
            pyth_trusted_signers: {
                let trused_signer = Binary::from_str(LAZER_TRUSTED_SIGNER).unwrap();
                btree_map! { trused_signer => Timestamp::from_nanos(u128::MAX) } // FIXME: what's the appropriate expiration time for this?
            },
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
