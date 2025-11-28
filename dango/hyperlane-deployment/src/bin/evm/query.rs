//! This script deploys the `HypNativeMetadata` contract as an upgradeable proxy
//! on Sepolia, configure the router, and sends 100 wei to a recipient on Dango.
//!
//! Prerequisite: create a `.env` file at the repository root, with the
//! following content:
//!
//! ```plain
//! INFURA_API_KEY="your_infura_api_key"
//! MNEMONIC="your_mnemonic"
//! ```

use {
    dango_hyperlane_deployment::{
        config,
        contract_bindings::{
            hyp_erc20_collateral::HypERC20Collateral, hyp_native::HypNative,
            ism::StaticMessageIdMultisigIsm,
        },
        evm::get_or_deploy_ism,
        setup,
    },
    dotenvy::dotenv,
};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let config = config::load_config()?;
    let evm_config = config.evm.get("sepolia").unwrap();

    let ism = evm_config.ism.clone();

    let (provider, _) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let ism_address = get_or_deploy_ism(&provider, &evm_config.hyperlane_deployments, ism).await?;

    println!("ISM address: {ism_address}");

    let warp_route_1 = evm_config.warp_routes[0].clone();
    let warp_route_2 = evm_config.warp_routes[1].clone();

    let hyp_native = HypNative::new(warp_route_1.address.unwrap(), &provider);
    let hyp_erc20_collateral =
        HypERC20Collateral::new(warp_route_2.proxy_address.unwrap(), &provider);
    let proxy = HypNative::new(warp_route_2.proxy_address.unwrap(), &provider);

    println!("Querying the warp route for the current ISM...");
    let hyp_native_ism = hyp_native.interchainSecurityModule().call().await?;
    println!("Native ISM: {hyp_native_ism}");

    println!("Querying the ERC20 collateral ISM for the current ISM...");
    let hyp_erc20_collateral_ism = hyp_erc20_collateral
        .interchainSecurityModule()
        .call()
        .await?;
    println!("ERC20 collateral ISM: {hyp_erc20_collateral_ism}");

    // Query the ISM for validators and threshold
    println!("Querying the new ISM for validators and threshold...");
    let validators_and_threshold = StaticMessageIdMultisigIsm::new(ism_address, &provider)
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
