use {
    dango_genesis::{build_genesis, build_rust_codes, GenesisUser},
    dango_types::{account_factory::Username, auth::Key},
    grug::{
        btree_map, Coin, Coins, Duration, HashExt, Inner, Json, JsonDeExt, JsonSerExt, Udec128,
        Uint128,
    },
    hex_literal::hex,
    home::home_dir,
    std::{env, fs, str::FromStr},
};

// See docs for the seed phrases of these keys.
const PK_OWNER: [u8; 33] =
    hex!("0278f7b7d93da9b5a62e28434184d1c337c2c28d4ced291793215ab6ee89d7fff8");
const PK_USER1: [u8; 33] =
    hex!("03bcf89d5d4f18048f0662d359d17a2dbbb08a80b1705bc10c0b953f21fb9e1911");
const PK_USER2: [u8; 33] =
    hex!("02d309ba716f271b1083e24a0b9d438ef7ae0505f63451bc1183992511b3b1d52d");
const PK_USER3: [u8; 33] =
    hex!("024bd61d80a2a163e6deafc3676c734d29f1379cb2c416a32b57ceed24b922eba0");

fn main() {
    // Read CLI arguments.
    // There should be exactly two arguments: the chain ID and genesis time.
    let mut args = env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "expected exactly two arguments");

    // Read wasm files.
    let codes = build_rust_codes();

    // Owner gets DG token and USDC; all others get USDC.
    let users = btree_map! {
        Username::from_str("owner").unwrap() => GenesisUser {
            key: Key::Secp256k1(PK_OWNER.into()),
            key_hash: PK_OWNER.hash160(),
            balances: [
                Coin::new("udng", 30_000_000_000).unwrap(),
                Coin::new("uusdc", 100_000_000_000_000).unwrap(),
            ]
            .try_into()
            .unwrap(),
        },
        Username::from_str("user1").unwrap() => GenesisUser {
            key: Key::Secp256k1(PK_USER1.into()),
            key_hash: PK_USER1.hash160(),
            balances: Coins::one("uusdc", 100_000_000_000_000).unwrap(),
        },
        Username::from_str("user2").unwrap() => GenesisUser {
            key: Key::Secp256k1(PK_USER2.into()),
            key_hash: PK_USER2.hash160(),
            balances: Coins::one("uusdc", 100_000_000_000_000).unwrap(),
        },
        Username::from_str("user3").unwrap() => GenesisUser {
            key: Key::Secp256k1(PK_USER3.into()),
            key_hash: PK_USER3.hash160(),
            balances: Coins::one("uusdc", 100_000_000_000_000).unwrap(),
        },
    };

    let (genesis_state, contracts, addresses) = build_genesis(
        codes,
        users,
        &Username::from_str("owner").unwrap(),
        "uusdc",
        Udec128::new_percent(25),                 // 0.25 uusdc per gas unit
        Some(Uint128::new(10_000_000)),           // 10 USDC
        Duration::from_seconds(7 * 24 * 60 * 60), // 1 week
    )
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
