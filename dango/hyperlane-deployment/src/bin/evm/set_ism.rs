//! This script updates the ISM on a deployed warp route.
//!
//! Prerequisite: create a `.env` file at the repository root, with the
//! following content:
//!
//! ```plain
//! INFURA_API_KEY="your_infura_api_key"
//! MNEMONIC="your_mnemonic"
//! ```

use {
    clap::Parser,
    dango_hyperlane_deployment::{
        config::{self},
        contract_bindings::hyp_native::HypNative,
        evm::get_or_deploy_ism,
        setup,
    },
    dotenvy::dotenv,
};

#[derive(Parser)]
#[command(name = "evm_set_ism")]
struct Args {
    #[arg(long)]
    config: String,
    #[arg(long)]
    deployments: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let config = config::load_config_from_path(&args.config)?;
    let evm_config = &config.evm;

    let deployments = config::load_deployments_from_path(&args.deployments)?;
    let evm_deployment = &deployments.evm;

    let (provider, _) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let ism = evm_config.ism.clone();

    let ism_address = get_or_deploy_ism(&provider, &evm_config.hyperlane_deployments, ism).await?;

    println!("ISM address: {ism_address}");

    let hyp_native = HypNative::new(evm_deployment.warp_routes[1].1.proxy_address, &provider);

    println!(
        "Setting ISM on warp route {:?} to {ism_address}...",
        *hyp_native.address()
    );
    let tx_hash = hyp_native
        .setInterchainSecurityModule(ism_address)
        .send()
        .await?
        .watch()
        .await?;
    println!("Done! Tx hash: {tx_hash}");

    Ok(())
}
