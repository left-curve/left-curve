use {
    cw_account::PubKey,
    cw_bank::Balance,
    cw_rs::{AdminOption, GenesisBuilder, SigningKey},
    cw_std::{Coin, Coins, Config, Uint128},
    home::home_dir,
    lazy_static::lazy_static,
    std::{env, path::PathBuf},
};

lazy_static! {
    static ref ARTIFACT_DIR: PathBuf = {
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../artifacts")
    };
    static ref KEYSTORE_DIR: PathBuf = {
        home_dir().unwrap().join(".cwcli/keys")
    };
}

const KEYSTORE_PASSWORD: &str = "123";

fn main() -> anyhow::Result<()> {
    // load two pubkeys from the keystore. we will register an account for each of them
    let test1 = SigningKey::from_file(&KEYSTORE_DIR.join("test1.json"), KEYSTORE_PASSWORD)?;
    let test2 = SigningKey::from_file(&KEYSTORE_DIR.join("test2.json"), KEYSTORE_PASSWORD)?;

    // create the genesis builder
    let mut builder = GenesisBuilder::new();

    // upload account wasm code
    let account_code_hash = builder.store_code(ARTIFACT_DIR.join("cw_account-aarch64.wasm"))?;

    // register two accounts
    let account1 = builder.instantiate(
        account_code_hash.clone(),
        cw_account::InstantiateMsg {
            pubkey: PubKey::Secp256k1(test1.pubkey().to_vec().into()),
        },
        b"test1".to_vec().into(),
        AdminOption::SetToSelf,
    )?;
    let account2 = builder.instantiate(
        account_code_hash.clone(),
        cw_account::InstantiateMsg {
            pubkey: PubKey::Secp256k1(test2.pubkey().to_vec().into()),
        },
        b"test2".to_vec().into(),
        AdminOption::SetToSelf,
    )?;

    // store and instantiate and bank contract
    // give account1 some initial balances
    let bank = builder.store_code_and_instantiate(
        ARTIFACT_DIR.join("cw_bank-aarch64.wasm"),
        cw_bank::InstantiateMsg {
            initial_balances: vec![Balance {
                address: account1.clone(),
                coins: Coins::from_vec_unchecked(vec![
                    Coin {
                        denom: "uatom".into(),
                        amount: Uint128::new(12345),
                    },
                    Coin {
                        denom: "uosmo".into(),
                        amount: Uint128::new(23456),
                    },
                ]),
            }],
        },
        b"bank".to_vec().into(),
        AdminOption::SetToNone,
    )?;

    // set config
    builder.set_config(Config {
        owner: None,
        bank:  bank.clone(),
    })?;

    // build the final genesis state and write to file
    builder.write_to_file(None)?;

    println!("✅ done!");
    println!("account1 : {account1}");
    println!("account2 : {account2}");
    println!("bank     : {bank}");

    Ok(())
}
