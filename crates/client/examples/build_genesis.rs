use {
    anyhow::anyhow,
    chrono::DateTime,
    grug_client::{AdminOption, GenesisBuilder, SigningKey},
    grug_types::{Coins, NonZero, Permission, Udec128, Uint128},
    home::home_dir,
    std::{path::PathBuf, str::FromStr},
};

fn main() -> anyhow::Result<()> {
    let home_dir = home_dir().ok_or(anyhow!("failed to find user's home directory"))?;
    let artifacts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../artifacts");

    let mut builder = GenesisBuilder::new()
        .with_genesis_time(DateTime::parse_from_rfc3339("2024-07-22T00:00:00.00000Z")?)
        .with_chain_id("grug-1");

    // Load two private keys that we created earlier.
    let k1 = SigningKey::from_file(home_dir.join(".grug/keys/test1.json"), "123")?;
    let k2 = SigningKey::from_file(home_dir.join(".grug/keys/test2.json"), "123")?;

    // Create two genesis accounts using the keys.
    let account_code_hash = builder.upload_file(artifacts_dir.join("grug_account.wasm"))?;
    let account1 = builder.instantiate(
        account_code_hash,
        &grug_account::InstantiateMsg {
            public_key: k1.public_key().into(),
        },
        "k1",
        Coins::new(),
        AdminOption::SetToSelf,
    )?;
    let account2 = builder.instantiate(
        account_code_hash,
        &grug_account::InstantiateMsg {
            public_key: k2.public_key().into(),
        },
        "k2",
        Coins::new(),
        AdminOption::SetToSelf,
    )?;

    // Deploy the bank contract; give the two genesis accounts some balances.
    let bank = builder.upload_file_and_instantiate(
        artifacts_dir.join("grug_bank.wasm"),
        &grug_bank::InstantiateMsg {
            initial_balances: [
                (
                    account1,
                    Coins::one("uatom", NonZero::new(Uint128::new(1_000_000))),
                ),
                (
                    account2,
                    Coins::one("uosmo", NonZero::new(Uint128::new(1_000_000))),
                ),
            ]
            .into(),
        },
        "bank",
        Coins::new(),
        AdminOption::SetToAddr(account1),
    )?;

    // Deploy the taxman contract.
    let taxman = builder.upload_file_and_instantiate(
        artifacts_dir.join("grug_taxman.wasm"),
        &grug_taxman::InstantiateMsg {
            config: grug_taxman::Config {
                fee_denom: "uatom".to_string(),
                fee_rate: Udec128::from_str("0.1")?,
            },
        },
        "taxman",
        Coins::new(),
        AdminOption::SetToAddr(account1),
    )?;

    // Build the genesis state and write to CometBFT genesis file.
    builder
        .set_owner(account1)
        .set_bank(bank)
        .set_taxman(taxman)
        .set_upload_permission(Permission::Everybody)
        .set_instantiate_permission(Permission::Everybody)
        .build_and_write_to_cometbft_genesis(home_dir.join(".cometbft/config/genesis.json"))
        .map(drop)
}
