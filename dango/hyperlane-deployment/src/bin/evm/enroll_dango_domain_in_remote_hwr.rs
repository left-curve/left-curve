//! This script enrolls the Dango domain in a remote Hyperlane Warp Route.
//!
//! Prerequisite: create a `.env` file at the repository root, with the
//! following content:
//!
//! ```plain
//! INFURA_API_KEY="your_infura_api_key"
//! SEPOLIA_MNEMONIC="your_mnemonic"
//! ```

use {
    alloy::primitives::Address,
    clap::Parser,
    dango_hyperlane_deployment::{config, evm::enroll_dango_domain, setup},
    dotenvy::dotenv,
};

const EVM_NETWORK: &str = "11155111";

#[derive(Parser)]
#[command(name = "evm_enroll_dango_domain")]
#[command(about = "Enrolls the Dango domain in a remote Hyperlane Warp Route")]
struct Args {
    /// The address of the warp route proxy contract
    #[arg(long)]
    warp_route_address: Address,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    // Load config
    let config = config::load_config()?;
    let evm_config = config
        .evm
        .get(EVM_NETWORK)
        .ok_or_else(|| anyhow::anyhow!("missing EVM config for network `{EVM_NETWORK}`"))?;

    // Setup Ethereum provider
    let (provider, _) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    // Setup Dango client
    let (dango_client, ..) = setup::setup_dango(&config.dango).await?;

    // Enroll dango domain in remote HWR
    enroll_dango_domain(
        &provider,
        &dango_client,
        args.warp_route_address,
        evm_config.chain_id,
    )
    .await
}
