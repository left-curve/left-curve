use {
    crate::{
        AccountOption, BankOption, DexOption, GatewayOption, GenesisOption, GenesisUser,
        GrugOption, HyperlaneOption, LendingOption, OracleOption, VestingOption, build_rust_codes,
    },
    dango_types::{
        account_factory::Username,
        auth::Key,
        bank::Metadata,
        constants::{PYTH_PRICE_SOURCES, btc, dango, eth, sol, usdc},
        dex::{CurveInvariant, PairParams, PairUpdate},
        gateway::{Remote, WithdrawalFee},
        taxman,
    },
    grug::{
        Bounded, Coin, ContractWrapper, Denom, Duration, HashExt, LengthBounded, Timestamp,
        Udec128, Uint128, btree_map, btree_set, coins,
    },
    hex_literal::hex,
    hyperlane_types::constants::{arbitrum, base, ethereum, optimism, solana},
    pyth_types::constants::GUARDIAN_SETS,
    std::{str::FromStr, sync::LazyLock},
};

pub const MOCK_CHAIN_ID: &str = "mock-1";

pub const MOCK_GENESIS_TIMESTAMP: Timestamp = Timestamp::from_days(365);

macro_rules! mock_user {
    ($username:ident, $vk:literal, $sk:literal) => {
        pub mod $username {
            use super::*;

            pub const USERNAME: LazyLock<Username> = LazyLock::new(|| Username::new_unchecked(stringify!($username)));
            pub const PUBLIC_KEY: [u8; 33] = hex!($vk);
            pub const PRIVATE_KEY: [u8; 32] = hex!($sk);
        }
    };
    ($({ $username:ident, $vk:literal, $sk:literal }),* $(,)?) => {
        $(
            mock_user!($username, $vk, $sk);
        )*
    };
}

mock_user! {
    {
        owner,
        "0278f7b7d93da9b5a62e28434184d1c337c2c28d4ced291793215ab6ee89d7fff8",
        "8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177"
    },
    {
        user1,
        "03bcf89d5d4f18048f0662d359d17a2dbbb08a80b1705bc10c0b953f21fb9e1911",
        "a5122c0729c1fae8587e3cc07ae952cb77dfccc049efd5be1d2168cbe946ca18"
    },
    {
        user2,
        "02d309ba716f271b1083e24a0b9d438ef7ae0505f63451bc1183992511b3b1d52d",
        "cac7b4ced59cf0bfb14c373272dfb3d4447c7cd5aea732ea6ff69e19f85d34c4"
    },
    {
        user3,
        "024bd61d80a2a163e6deafc3676c734d29f1379cb2c416a32b57ceed24b922eba0",
        "cf6bb15648a3a24976e2eeffaae6201bc3e945335286d273bb491873ac7c3141"
    },
    {
        user4,
        "024a23e7a6f85e942a4dbedb871c366a1fdad6d0b84e670125991996134c270df2",
        "126b714bfe7ace5aac396aa63ff5c92c89a2d894debe699576006202c63a9cf6"
    },
    {
        user5,
        "03da86b1cd6fd20350a0b525118eef939477c0fe3f5052197cd6314ed72f9970ad",
        "fe55076e4b2c9ffea813951406e8142fefc85183ebda6222500572b0a92032a7"
    },
    {
        user6,
        "03428b179a075ff2142453c805a71a63b232400cc33c8e8437211e13e2bd1dec4c",
        "4d3658519dd8a8227764f64c6724b840ffe29f1ca456f5dfdd67f834e10aae34"
    },
    {
        user7,
        "028d4d7265d5838190842ada2573ef9edfc978dec97ca59ce48cf1dd19352a4407",
        "82de24ba8e1bc4511ae10ce3fbe84b4bb8d7d8abc9ba221d7d3cf7cd0a85131f"
    },
    {
        user8,
        "02a888b140a836cd71a5ef9bc7677a387a2a4272343cf40722ab9e85d5f8aa21bd",
        "ca956fcf6b0f32975f067e2deaf3bc1c8632be02ed628985105fd1afc94531b9"
    },
    {
        user9,
        "0230f93baa8e1dbe40a928144ec2144eed902c94b835420a6af4aafd2e88cb7b52",
        "c0d853951557d3bdec5add2ca8e03983fea2f50c6db0a45977990fb7b0c569b3"
    }
}

impl GenesisOption<ContractWrapper> {
    pub fn preset_test() -> Self {
        GenesisOption {
            codes: build_rust_codes(),
            grug: GrugOption {
                owner_username: owner::USERNAME.clone(),
                fee_cfg: taxman::Config {
                    fee_denom: usdc::DENOM.clone(),
                    fee_rate: Udec128::new_percent(25), // 0.25 uusdc per gas unit
                },
                max_orphan_age: Duration::from_weeks(1),
            },
            account: AccountOption {
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
            },
            bank: BankOption {
                metadatas: btree_map! {
                    dango::DENOM.clone() => Metadata {
                        name: LengthBounded::new_unchecked("Dango".to_string()),
                        symbol: LengthBounded::new_unchecked("DGX".to_string()),
                        decimals: 6,
                        description: Some(LengthBounded::new_unchecked("Native token of Dango".to_string()))
                    }
                },
            },
            dex: DexOption {
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
            },
            gateway: GatewayOption {
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
            },
            hyperlane: HyperlaneOption {
                local_domain: 88888888,
                ism_validator_sets: btree_map! {
                    // TODO
                },
                va_announce_fee_per_byte: Coin {
                    denom: usdc::DENOM.clone(),
                    amount: Uint128::new(100),
                },
            },
            lending: LendingOption {
                markets: btree_map! {
                    // TODO
                },
            },
            oracle: OracleOption {
                pyth_price_sources: PYTH_PRICE_SOURCES.clone(),
                wormhole_guardian_sets: GUARDIAN_SETS.clone(),
            },
            vesting: VestingOption {
                unlocking_cliff: Duration::from_weeks(4 * 9), // ~9 months
                unlocking_period: Duration::from_weeks(4 * 27), // ~27 months
            },
        }
    }
}
