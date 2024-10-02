use {
    anyhow::anyhow,
    dango_genesis::{build_genesis, Codes, GenesisUser},
    dango_types::{account_factory::Username, auth::Key},
    grug::{Coins, HashExt, Json, JsonDeExt, JsonSerExt, Udec128, Uint128},
    k256::{
        ecdsa::{SigningKey, VerifyingKey},
        elliptic_curve::rand_core::OsRng,
    },
    std::{fs, path::PathBuf, str::FromStr},
};

const COMETBFT_GENESIS_PATH: &str = "/Users/larry/.cometbft/config/genesis.json";

// For demo purpose, we simply generate random genesis users. For production,
// these are typically read from a file.
fn random_genesis_user(username: &str, balances: Coins) -> anyhow::Result<(Username, GenesisUser)> {
    let username = Username::from_str(username)?;

    let sk = SigningKey::random(&mut OsRng);
    let pk = VerifyingKey::from(&sk).to_sec1_bytes().to_vec();
    let key_hash = pk.hash160();
    let key = Key::Secp256k1(pk.try_into()?);

    let user = GenesisUser {
        key,
        key_hash,
        balances,
    };

    Ok((username, user))
}

fn main() -> anyhow::Result<()> {
    let codes = {
        let artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts");

        let account_factory = fs::read(artifacts_dir.join("dango_account_factory.wasm"))?;
        let account_spot = fs::read(artifacts_dir.join("app_account_spot.wasm"))?;
        let account_safe = fs::read(artifacts_dir.join("app_account_safe.wasm"))?;
        let amm = fs::read(artifacts_dir.join("app_amm.wasm"))?;
        let bank = fs::read(artifacts_dir.join("app_bank.wasm"))?;
        let ibc_transfer = fs::read(artifacts_dir.join("app_mock_ibc_transfer.wasm"))?;
        let taxman = fs::read(artifacts_dir.join("app_taxman.wasm"))?;
        let token_factory = fs::read(artifacts_dir.join("app_token_factory.wasm"))?;

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

    let users = [
        random_genesis_user("fee_recipient", Coins::new())?,
        random_genesis_user("owner", Coins::new())?,
        random_genesis_user("relayer", Coins::one("uusdc", 100_000_000_000_000)?)?,
    ]
    .into();

    let (genesis_state, contracts, addresses) = build_genesis(
        codes,
        users,
        &Username::from_str("owner")?,
        &Username::from_str("fee_recipient")?,
        "uusdc",
        Udec128::new_percent(25),
        Uint128::new(10_000_000),
    )?;

    println!("genesis_state = {}", genesis_state.to_json_string_pretty()?);
    println!("\ncontracts = {}", contracts.to_json_string_pretty()?);
    println!("\naddresses = {}\n", addresses.to_json_string_pretty()?);

    let cometbft_genesis_raw = fs::read(COMETBFT_GENESIS_PATH)?;
    let mut cometbft_genesis: Json = cometbft_genesis_raw.deserialize_json()?;

    cometbft_genesis
        .as_object_mut()
        .ok_or(anyhow!("cometbft genesis file isn't an object"))?
        .insert("app_state".to_string(), genesis_state.to_json_value()?);

    fs::write(
        COMETBFT_GENESIS_PATH,
        cometbft_genesis.to_json_string_pretty()?,
    )?;

    Ok(())
}
