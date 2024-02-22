use {
    cw_account::PublicKey,
    cw_bank::Balance,
    cw_rs::{AdminOption, GenesisBuilder, SigningKey},
    cw_std::{Coin, Coins, Config, Empty, Permission, Uint128},
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

    // store and instantiate the account factory contract
    let account_factory = builder.store_code_and_instantiate(
        ARTIFACT_DIR.join("cw_account_factory-aarch64.wasm"),
        cw_account_factory::InstantiateMsg {},
        b"account-factory".to_vec().into(),
        AdminOption::SetToNone,
    )?;

    // register two accounts
    let account1 = builder.register_account(
        account_factory.clone(),
        account_code_hash.clone(),
        PublicKey::Secp256k1(test1.public_key().to_vec().into()),
    )?;
    let account2 = builder.register_account(
        account_factory.clone(),
        account_code_hash,
        PublicKey::Secp256k1(test2.public_key().to_vec().into()),
    )?;

    // store and instantiate the bank contract
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

    // store and instantiate mock cron contract
    let cron = builder.store_code_and_instantiate(
        ARTIFACT_DIR.join("cw_mock_cron-aarch64.wasm"),
        Empty {},
        b"cron".to_vec().into(),
        AdminOption::SetToNone,
    )?;

    // set config
    builder.set_config(Config {
        owner:                  None,
        bank:                   bank.clone(),
        begin_blockers:         vec![cron.clone()],
        end_blockers:           vec![cron.clone()],
        store_code_permission:  Permission::Somebodies([account1.clone()].into()),
        instantiate_permission: Permission::Somebodies([account1.clone(), account_factory.clone()].into()),
    })?;

    // build the final genesis state and write to file
    builder.write_to_file(None)?;

    println!("✅ done!");
    println!("account-factory : {account_factory}");
    println!("account1        : {account1}");
    println!("account2        : {account2}");
    println!("bank            : {bank}");
    println!("cron            : {cron}");

    Ok(())
}
