use {
    cw_account::PublicKey,
    cw_rs::{AdminOption, GenesisBuilder, SigningKey},
    cw_std::{Coin, Coins, Config, Empty, Permission, Permissions, Uint128},
    home::home_dir,
    lazy_static::lazy_static,
    std::{collections::BTreeMap, env, path::PathBuf},
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
    // load a few pubkeys from the keystore. we will register an account for each of them
    let test1 = SigningKey::from_file(&KEYSTORE_DIR.join("test1.json"), KEYSTORE_PASSWORD)?;
    let test2 = SigningKey::from_file(&KEYSTORE_DIR.join("test2.json"), KEYSTORE_PASSWORD)?;
    let test3 = SigningKey::from_file(&KEYSTORE_DIR.join("test3.json"), KEYSTORE_PASSWORD)?;

    // create the genesis builder
    let mut builder = GenesisBuilder::new();

    // upload account wasm code
    let account_code_hash = builder.upload(ARTIFACT_DIR.join("cw_account-aarch64.wasm"))?;

    // store and instantiate the account factory contract
    let account_factory = builder.upload_and_instantiate(
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
        account_code_hash.clone(),
        PublicKey::Secp256k1(test2.public_key().to_vec().into()),
    )?;
    let account3 = builder.register_account(
        account_factory.clone(),
        account_code_hash,
        PublicKey::Secp256k1(test3.public_key().to_vec().into()),
    )?;

    // store and instantiate the bank contract
    // give each account some initial balances
    let coins = Coins::from_vec_unchecked(vec![
        Coin {
            denom: "uatom".into(),
            amount: Uint128::new(20000),
        },
        Coin {
            denom: "uosmo".into(),
            amount: Uint128::new(20000),
        },
    ]);
    let bank = builder.upload_and_instantiate(
        ARTIFACT_DIR.join("cw_bank-aarch64.wasm"),
        cw_bank::InstantiateMsg {
            initial_balances: BTreeMap::from([
                (account1.clone(), coins.clone()),
                (account2.clone(), coins.clone()),
                (account3.clone(), coins.clone()),
            ]),
        },
        b"bank".to_vec().into(),
        AdminOption::SetToNone,
    )?;

    // store and instantiate mock cron contract
    let cron = builder.upload_and_instantiate(
        ARTIFACT_DIR.join("cw_mock_cron-aarch64.wasm"),
        Empty {},
        b"cron".to_vec().into(),
        AdminOption::SetToNone,
    )?;

    // upload the solo machine code
    let solomachine_hash = builder.upload(ARTIFACT_DIR.join("cw_ibc_solomachine-aarch64.wasm"))?;

    // set config
    let permissions = Permissions {
        upload:            Permission::Somebodies([account1.clone(), account2.clone(), account3.clone()].into()),
        instantiate:       Permission::Everybody,
        create_client:     Permission::Everybody,
        create_connection: Permission::Everybody,
        create_channel:    Permission::Everybody,
    };
    builder.set_config(Config {
        owner:           None,
        bank:            bank.clone(),
        begin_blockers:  vec![cron.clone()],
        end_blockers:    vec![cron.clone()],
        allowed_clients: [solomachine_hash].into(),
        permissions,
    })?;

    // build the final genesis state and write to file
    builder.write_to_file(None)?;

    println!("✅ done!");
    println!("account-factory : {account_factory}");
    println!("account1        : {account1}");
    println!("account2        : {account2}");
    println!("account3        : {account3}");
    println!("bank            : {bank}");
    println!("cron            : {cron}");

    Ok(())
}
