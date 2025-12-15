use {
    anyhow::anyhow,
    dango_hyperlane_deployment::{config, dango, setup},
    dotenvy::dotenv,
};

const REMOTE_CHAIN_ID: &str = "sepolia";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let config = config::load_config()?;

    let evm_config = config
        .evm
        .get(REMOTE_CHAIN_ID)
        .ok_or_else(|| anyhow!("EVM config not found for `{REMOTE_CHAIN_ID}`"))?;

    let (dango_client, mut dango_owner) = setup::setup_dango(&config.dango).await?;

    dango::set_ism_validator_set(&dango_client, &config, &mut dango_owner, evm_config).await
}
