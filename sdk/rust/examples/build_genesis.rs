use {
    anyhow::anyhow,
    cw_account::PubKey,
    cw_bank::Balance,
    cw_rs::{AdminOption, GenesisBuilder, Keyring},
    cw_std::{Coin, Coins, Config, Uint128},
    home::home_dir,
    std::{env, path::PathBuf},
};

fn main() -> anyhow::Result<()> {
    let artifact_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../artifacts");

    // open the keyring. we will register accounts for two of the keys
    let home_dir = home_dir().ok_or(anyhow!("Failed to find user home directory"))?;
    let keyring = Keyring::open(home_dir.join(".cwcli/keys"))?;

    let mut builder = GenesisBuilder::new();

    // upload account code and register two accounts
    let account_code_hash = builder.store_code(artifact_dir.join("cw_bank-aarch64.wasm"))?;

    let key1 = keyring.get("test1")?;
    let account1 = builder.instantiate(
        account_code_hash.clone(),
        cw_account::InstantiateMsg {
            pubkey: PubKey::Secp256k1(key1.verifying_key().to_sec1_bytes().to_vec().into()),
        },
        b"test1".to_vec().into(),
        Coins::new_empty(),
        AdminOption::SetToSelf,
    )?;

    let key2 = keyring.get("test2")?;
    let account2 = builder.instantiate(
        account_code_hash.clone(),
        cw_account::InstantiateMsg {
            pubkey: PubKey::Secp256k1(key2.verifying_key().to_sec1_bytes().to_vec().into()),
        },
        b"test2".to_vec().into(),
        Coins::new_empty(),
        AdminOption::SetToSelf,
    )?;

    // upload bank code and register account
    // give account1 some initial balances
    let bank = builder.store_code_and_instantiate(
        artifact_dir.join("cw_bank-aarch64.wasm"),
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
        Coins::new_empty(),
        AdminOption::SetToNone,
    )?;

    // set config
    builder.set_config(Config {
        owner: None,
        bank:  bank.clone(),
    })?;

    // build the final genesis state and write to file
    builder.write_to_file(None)?;

    println!("done!");
    println!("account1 : {account1}");
    println!("account2 : {account2}");
    println!("bank     : {bank}");

    Ok(())
}
