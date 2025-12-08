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
        config::{
            self,
            evm::{WarpRoute, WarpRouteType},
        },
        evm::{deploy_proxy_admin, deploy_warp_route_and_update_deployment, get_or_deploy_ism},
        setup,
    },
    dotenvy::dotenv,
};

// The kind of warp route to deploy.
const WARP_ROUTE_TYPE: WarpRouteType = WarpRouteType::Native;
// The symbol to use as subdenom for the token on Dango.
const SYMBOL: &str = "sepoliaETH";

const EVM_NETWORK: &str = "sepolia";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let mut config = config::load_config()?;
    let mut deployments = config::load_deployments()?;

    let (dango_client, ..) = setup::setup_dango(&config.dango).await?;

    let evm_config = config.evm.get_mut(EVM_NETWORK).unwrap();
    let (provider, owner) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let maybe_deployment = deployments.evm.get(EVM_NETWORK);

    let mut evm_deployment = match maybe_deployment {
        Some(deployment) => deployment.clone(),
        None => {
            let proxy_admin_address = deploy_proxy_admin(&provider).await?;
            config::EVMDeployment {
                proxy_admin_address,
                warp_routes: vec![],
            }
        },
    };

    let warp_route = WarpRoute {
        warp_route_type: WARP_ROUTE_TYPE.clone(),
        symbol: SYMBOL.to_string(),
    };

    // Get or deploy the ISM
    let ism = get_or_deploy_ism(
        &provider,
        &evm_config.hyperlane_deployments,
        evm_config.ism.clone(),
    )
    .await?;

    deploy_warp_route_and_update_deployment(
        &provider,
        &dango_client,
        &warp_route,
        owner,
        Some(ism),
        evm_config,
        &mut evm_deployment,
    )
    .await?;

    // Update the deployments with the new deployment
    deployments
        .evm
        .insert(EVM_NETWORK.to_string(), evm_deployment);

    // Save the updated deployments
    println!("Saving updated deployments...");
    config::save_deployments(&deployments)?;

    println!("Done!");

    Ok(())
}
