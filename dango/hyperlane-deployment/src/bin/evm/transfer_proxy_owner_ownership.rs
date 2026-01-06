//! This script transfers ownership of the ProxyAdmin contract to a new owner (e.g., a multi-sig).
//!
//! Prerequisite: create a `.env` file at the repository root, with the
//! following content:
//!
//! ```plain
//! INFURA_API_KEY="your_infura_api_key"
//! MNEMONIC="your_mnemonic"
//! ```

use {
    alloy::primitives::Address,
    clap::Parser,
    dango_hyperlane_deployment::{config, evm::transfer_proxy_owner_ownership, setup},
    dotenvy::dotenv,
};

#[derive(Parser)]
#[command(name = "transfer_proxy_owner_ownership")]
#[command(about = "Transfers ownership of the ProxyAdmin contract to a new owner")]
struct Args {
    /// Path to the config file
    #[arg(long)]
    config: String,
    /// Path to the deployments file
    #[arg(long)]
    deployments: String,
    /// The EVM chain ID to transfer ownership on
    #[arg(long)]
    chain_id: String,
    /// The new owner address (overrides multi_sig_address from config if provided)
    #[arg(long)]
    new_owner: Option<Address>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let config = config::load_config_from_path(&args.config)?;
    let deployments = config::load_deployments_from_path(&args.deployments)?;

    let evm_config = config
        .evm
        .get(&args.chain_id)
        .ok_or_else(|| anyhow::anyhow!("EVM config not found for chain_id: {}", args.chain_id))?;

    let evm_deployment = deployments.evm.get(&args.chain_id).ok_or_else(|| {
        anyhow::anyhow!("EVM deployment not found for chain_id: {}", args.chain_id)
    })?;

    let (provider, _) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    // Use the new_owner from args if provided, otherwise use multi_sig_address from config
    let new_owner = args
        .new_owner
        .or(evm_config.multi_sig_address)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No new owner specified. Provide --new-owner or set multi_sig_address in config"
            )
        })?;

    transfer_proxy_owner_ownership(&provider, evm_deployment.proxy_admin_address, new_owner)
        .await?;

    println!(
        "Successfully transferred ProxyAdmin ownership to {} on chain {}",
        new_owner, args.chain_id
    );

    Ok(())
}
