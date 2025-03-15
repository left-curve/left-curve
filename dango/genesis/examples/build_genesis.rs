use {
    dango_genesis::{build_genesis, build_rust_codes, GenesisConfig, GenesisUser},
    dango_types::{
        account_factory::Username,
        auth::Key,
        constants::{
            BTC_DENOM, DANGO_DENOM, ETH_DENOM, GUARDIAN_SETS, PYTH_PRICE_SOURCES, SOL_DENOM,
            USDC_DENOM,
        },
        dex::{CurveInvariant, PairParams, PairUpdate},
        taxman,
    },
    grug::{
        btree_map, coins, Bounded, Coin, Denom, Duration, HashExt, Inner, Json, JsonDeExt,
        JsonSerExt, Udec128,
    },
    hex_literal::hex,
    home::home_dir,
    std::{env, fs, str::FromStr},
};

// Private keys of devnet test accounts.
// See docs for the seed phrases of these keys.
const PK_OWNER: [u8; 33] =
    hex!("0278f7b7d93da9b5a62e28434184d1c337c2c28d4ced291793215ab6ee89d7fff8");
const PK_USER1: [u8; 33] =
    hex!("03bcf89d5d4f18048f0662d359d17a2dbbb08a80b1705bc10c0b953f21fb9e1911");
const PK_USER2: [u8; 33] =
    hex!("02d309ba716f271b1083e24a0b9d438ef7ae0505f63451bc1183992511b3b1d52d");
const PK_USER3: [u8; 33] =
    hex!("024bd61d80a2a163e6deafc3676c734d29f1379cb2c416a32b57ceed24b922eba0");
const PK_USER4: [u8; 33] =
    hex!("024a23e7a6f85e942a4dbedb871c366a1fdad6d0b84e670125991996134c270df2");
const PK_USER5: [u8; 33] =
    hex!("03da86b1cd6fd20350a0b525118eef939477c0fe3f5052197cd6314ed72f9970ad");
const PK_USER6: [u8; 33] =
    hex!("03428b179a075ff2142453c805a71a63b232400cc33c8e8437211e13e2bd1dec4c");
const PK_USER7: [u8; 33] =
    hex!("028d4d7265d5838190842ada2573ef9edfc978dec97ca59ce48cf1dd19352a4407");
const PK_USER8: [u8; 33] =
    hex!("02a888b140a836cd71a5ef9bc7677a387a2a4272343cf40722ab9e85d5f8aa21bd");
const PK_USER9: [u8; 33] =
    hex!("0230f93baa8e1dbe40a928144ec2144eed902c94b835420a6af4aafd2e88cb7b52");

fn main() {
    // Read CLI arguments.
    // There should be exactly two arguments: the chain ID and genesis time.
    let mut args = env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        3,
        "expecting exactly two positional arguments: chain ID and genesis time. example:\n$ cargo run -p dango-genesis --example build_genesis -- dev-5 2025-02-01T00:00:00Z",
    );

    let (genesis_state, contracts, addresses) = build_genesis(GenesisConfig {
        codes: build_rust_codes(),
        users: btree_map! {
            Username::from_str("owner").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_OWNER.into()),
                key_hash: PK_OWNER.hash256(),
                balances: coins! {
                    DANGO_DENOM.clone() => 30_000_000_000,
                    USDC_DENOM.clone()  => 100_000_000_000_000,
                },
            },
            Username::from_str("user1").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER1.into()),
                key_hash: PK_USER1.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user2").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER2.into()),
                key_hash: PK_USER2.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user3").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER3.into()),
                key_hash: PK_USER3.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user4").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER4.into()),
                key_hash: PK_USER4.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user5").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER5.into()),
                key_hash: PK_USER5.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user6").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER6.into()),
                key_hash: PK_USER6.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user7").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER7.into()),
                key_hash: PK_USER7.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user8").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER8.into()),
                key_hash: PK_USER8.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
            Username::from_str("user9").unwrap() => GenesisUser {
                key: Key::Secp256k1(PK_USER9.into()),
                key_hash: PK_USER9.hash256(),
                balances: coins! { USDC_DENOM.clone() => 100_000_000_000_000 },
            },
        },
        account_factory_minimum_deposit: coins! { USDC_DENOM.clone() => 10_000_000 },
        owner: Username::from_str("owner").unwrap(),
        fee_cfg: taxman::Config {
            fee_denom: USDC_DENOM.clone(),
            fee_rate: Udec128::new_percent(25), // 0.25 uusdc per gas unit
        },
        max_orphan_age: Duration::from_weeks(1),
        metadatas: btree_map! {},
        pairs: vec![
            PairUpdate {
                base_denom: DANGO_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                },
            },
            PairUpdate {
                base_denom: BTC_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                },
            },
            PairUpdate {
                base_denom: ETH_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/eth/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                },
            },
            PairUpdate {
                base_denom: SOL_DENOM.clone(),
                quote_denom: USDC_DENOM.clone(),
                params: PairParams {
                    lp_denom: Denom::from_str("dex/pool/sol/usdc").unwrap(),
                    curve_invariant: CurveInvariant::Xyk,
                    swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(30)),
                },
            },
        ],
        markets: btree_map! {},
        price_sources: PYTH_PRICE_SOURCES.clone(),
        unlocking_cliff: Duration::from_weeks(4 * 9), // ~9 months
        unlocking_period: Duration::from_weeks(4 * 27), // ~27 months
        wormhole_guardian_sets: GUARDIAN_SETS.clone(),
        hyperlane_local_domain: 88888888,
        hyperlane_ism_validator_sets: btree_map! {},
        hyperlane_va_announce_fee_per_byte: Coin::new(USDC_DENOM.clone(), 100).unwrap(),
        warp_routes: btree_map! {},
    })
    .unwrap();

    println!(
        "genesis_state = {}",
        genesis_state.to_json_string_pretty().unwrap()
    );
    println!(
        "\ncontracts = {}",
        contracts.to_json_string_pretty().unwrap()
    );
    println!(
        "\naddresses = {}\n",
        addresses.to_json_string_pretty().unwrap()
    );

    let cometbft_genesis_path = home_dir().unwrap().join(".cometbft/config/genesis.json");

    let mut cometbft_genesis = fs::read(&cometbft_genesis_path)
        .unwrap()
        .deserialize_json::<Json>()
        .unwrap();

    let map = cometbft_genesis.as_object_mut().unwrap();
    map.insert("genesis_time".into(), args.pop().unwrap().into());
    map.insert("chain_id".into(), args.pop().unwrap().into());
    map.insert(
        "app_state".into(),
        genesis_state.to_json_value().unwrap().into_inner(),
    );

    fs::write(
        cometbft_genesis_path,
        cometbft_genesis.to_json_string_pretty().unwrap(),
    )
    .unwrap();
}
