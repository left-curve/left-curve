use {
    grug_rs::{AdminOption, Client, SigningKey, SigningOptions},
    grug::{Addr, Coin, Coins, Uint128},
    home::home_dir,
    lazy_static::lazy_static,
    std::{env, fs, path::PathBuf, str::FromStr, thread, time::Duration},
};

lazy_static! {
    static ref ARTIFACT_DIR: PathBuf = {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../artifacts")
    };
    static ref KEYSTORE_DIR: PathBuf = {
        home_dir().unwrap().join(".cwcli/keys")
    };
    static ref USER: Addr = {
        Addr::from_str("0xaa07224072e473b6ef413f4cbf32a12baf94472a162b0b51435c8995c6d5ecdd").unwrap()
    };
    static ref BANK: Addr = {
        Addr::from_str("0xc98ae1b34d8aa3860c026a3499624f6c2e35377e768d8c56d3f0f5b7ab27d377").unwrap()
    };
}

const KEYSTORE_PASSWORD: &str = "123";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // load signing key
    let test1 = SigningKey::from_file(&KEYSTORE_DIR.join("test1.json"), KEYSTORE_PASSWORD)?;
    let sign_opts = SigningOptions {
        signing_key: test1,
        sender:      USER.clone(),
        chain_id:    None,
        sequence:    None,
    };

    // create client
    let client = Client::connect("http://127.0.0.1:26657")?;

    // store and instantiate token wrapper contract
    let wrapper_wasm = fs::read(ARTIFACT_DIR.join("grug_mock_token_wrapper-aarch64.wasm"))?;
    let (wrapper, tx1) = client.upload_and_instantiate(
        wrapper_wasm.into(),
        &grug_mock_token_wrapper::InstantiateMsg {
            bank: BANK.clone(),
        },
        b"wrapper".to_vec().into(),
        Coins::new_empty(),
        AdminOption::SetToNone,
        &sign_opts,
    )
    .await?;
    println!("\nwrapper contract instantiated!");
    println!("address: {wrapper}");
    println!("txhash: {}", tx1.hash);

    // wait 2 seconds for tx to settle
    thread::sleep(Duration::from_secs(2));

    // query the user's balances
    let balances_before = client.query_balances(USER.clone(), None, None, None).await?;
    println!("\nuser balances before wrapping:\n{}", serde_json::to_string_pretty(&balances_before)?);

    // wrap some tokens
    let tx2 = client.transfer(
        wrapper,
        Coins::try_from(vec![
            Coin {
                denom: "uatom".into(),
                amount: Uint128::new(888),
            },
            Coin {
                denom: "uosmo".into(),
                amount: Uint128::new(999),
            },
        ])?,
        &sign_opts,
    )
    .await?;
    println!("\ntokens wrapped!");
    println!("txhash: {}", tx2.hash);

    // wait 2 seconds for tx to settle
    thread::sleep(Duration::from_secs(2));

    // query the user's balances again
    let balances_after = client.query_balances(USER.clone(), None, None, None).await?;
    println!("\nuser balances after wrapping:\n{}", serde_json::to_string_pretty(&balances_after)?);

    Ok(())
}
