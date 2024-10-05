use {
    dango_genesis::{build_genesis, Codes, GenesisUser},
    dango_types::{account_factory::Username, auth::Key},
    grug::{btree_map, Coin, Coins, HashExt, Json, JsonDeExt, JsonSerExt, Udec128, Uint128},
    hex_literal::hex,
    std::{env, fs, path::PathBuf, str::FromStr},
};

const COMETBFT_GENESIS_PATH: &str = "/Users/larry/.cometbft/config/genesis.json";

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
    let codes = {
        let artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts");

        let account_factory = fs::read(artifacts_dir.join("dango_account_factory.wasm")).unwrap();
        let account_spot = fs::read(artifacts_dir.join("dango_account_spot.wasm")).unwrap();
        let account_safe = fs::read(artifacts_dir.join("dango_account_safe.wasm")).unwrap();
        let amm = fs::read(artifacts_dir.join("dango_amm.wasm")).unwrap();
        let bank = fs::read(artifacts_dir.join("dango_bank.wasm")).unwrap();
        let ibc_transfer = fs::read(artifacts_dir.join("dango_ibc_transfer.wasm")).unwrap();
        let taxman = fs::read(artifacts_dir.join("dango_taxman.wasm")).unwrap();
        let token_factory = fs::read(artifacts_dir.join("dango_token_factory.wasm")).unwrap();

        Codes {
            account_factory,
            account_spot,
            account_safe,
            amm,
            bank,
            ibc_transfer,
            taxman,
            token_factory,
        }
    };

    // Owner gets DG token and USDC; all others get USDC.
    let users = btree_map! {
        Username::from_str("owner").unwrap() => GenesisUser {
            key: Key::Secp256k1(PK_OWNER.into()),
            key_hash: PK_OWNER.hash160(),
            balances: [
                Coin::new("udg", 30_000_000_000).unwrap(),
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
        // TODO: Use owner as fee recipient for now. This should be replaced
        // with a contract.
        &Username::from_str("owner").unwrap(),
        "uusdc",
        Udec128::new_percent(25), // 0.25 uusdc per gas unit
        Uint128::new(10_000_000), // 10 USDC
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

    let mut cometbft_genesis = fs::read(COMETBFT_GENESIS_PATH)
        .unwrap()
        .deserialize_json::<Json>()
        .unwrap();

    let map = cometbft_genesis.as_object_mut().unwrap();
    map.insert("genesis_time".into(), args.pop().unwrap().into());
    map.insert("chain_id".into(), args.pop().unwrap().into());
    map.insert("app_state".into(), genesis_state.to_json_value().unwrap());

    fs::write(
        COMETBFT_GENESIS_PATH,
        cometbft_genesis.to_json_string_pretty().unwrap(),
    )
    .unwrap();
}
