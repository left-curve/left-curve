//! This script queries ISM validators and threshold for a deployed warp route.
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
        config,
        contract_bindings::{hyp_native::HypNative, ism::StaticMessageIdMultisigIsm},
        evm::get_or_deploy_ism,
        setup,
    },
    dotenvy::dotenv,
};

#[derive(Parser)]
#[command(name = "evm_query")]
struct Args {
    #[arg(long)]
    config: String,
    #[arg(long)]
    deployments: String,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let config = config::load_config_from_path(&args.config)?;
    let evm_config = &config.evm;

    let deployments = config::load_deployments_from_path(&args.deployments)?;
    let evm_deployment = &deployments.evm;

    let ism = evm_config.ism.clone();

    let (provider, _) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let ism_address = get_or_deploy_ism(&provider, &evm_config.hyperlane_deployments, ism).await?;

    println!("ISM address: {ism_address}");

    let warp_route_2 = evm_deployment.warp_routes[1].clone();

    let proxy = HypNative::new(warp_route_2.1.proxy_address, &provider);

    println!("Querying the warp route for the current ISM...");
    let hyp_native_ism = proxy.interchainSecurityModule().call().await?;
    println!("Native ISM: {hyp_native_ism}");

    // Query the ISM for validators and threshold
    println!("Querying the new ISM for validators and threshold...");
    let validators_and_threshold = StaticMessageIdMultisigIsm::new(hyp_native_ism, &provider)
        .validatorsAndThreshold(b"".to_vec().into())
        .call()
        .await?;
    println!("Validators: {:?}", validators_and_threshold._0);
    println!("Threshold: {:?}", validators_and_threshold._1);

    println!("Querying the HypNative ISM for validators and threshold...");
    let validators_and_threshold = StaticMessageIdMultisigIsm::new(hyp_native_ism, &provider)
        .validatorsAndThreshold(b"".to_vec().into())
        .call()
        .await?;
    println!("Validators: {:?}", validators_and_threshold._0);
    println!("Threshold: {:?}", validators_and_threshold._1);

    println!("Querying the proxy for the current ISM...");
    let proxy_ism = proxy.interchainSecurityModule().call().await?;
    println!("Current ISM: {proxy_ism}");

    Ok(())
}
