use {
    anyhow::anyhow,
    cw_account::PubKey,
    cw_bank::Balance,
    cw_keyring::Keyring,
    cw_std::{Coin, Coins, Config, Uint128},
    cw_utils::{AdminOption, GenesisBuilder},
    home::home_dir,
    std::{env, fs, path::PathBuf},
};

fn main() -> anyhow::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);

    // load wasm binaries
    let account_wasm = manifest_dir.join("../../artifacts/cw_account-aarch64.wasm");
    let bank_wasm = manifest_dir.join("../../artifacts/cw_bank-aarch64.wasm");

    // open the keyring. we will register accounts for two of the keys
    let home_dir = home_dir().ok_or(anyhow!("Failed to find user home directory"))?;
    let keyring = Keyring::open(home_dir.join(".cwcli/keys"))?;

    let mut builder = GenesisBuilder::new();

    // upload account code and register two accounts
    let account_code_hash = builder.store_code(account_wasm)?;

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
    let _account2 = builder.instantiate(
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
    let bank_addr = builder.store_code_and_instantiate(
        bank_wasm,
        cw_bank::InstantiateMsg {
            initial_balances: vec![Balance {
                address: account1,
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
        bank: bank_addr,
    })?;

    // build the final genesis state
    let genesis_state = builder.finalize()?;
    let genesis_state_str = serde_json::to_string_pretty(&genesis_state)?;

    // prepare to write
    let testdata_dir = manifest_dir.join("testdata");
    if !testdata_dir.exists() {
        fs::create_dir_all(&testdata_dir)?;
    }

    // write the genesis state to file
    // you can paste it to the `app_state` section of ~/.cometbft/config/genesis.json
    let out_path = testdata_dir.join("genesis_state.json");
    fs::write(&out_path, genesis_state_str.as_bytes())?;

    println!("✅ Genesis state written to {out_path:?}");

    Ok(())
}
