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
    clap::Parser,
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

#[derive(Parser)]
#[command(name = "evm_deploy_hwr_sepolia")]
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
    let deployments = config::load_deployments_from_path(&args.deployments).ok();

    let (dango_client, ..) = setup::setup_dango(&config.dango).await?;

    let evm_config = &config.evm;
    let (provider, owner) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let mut evm_deployment = match deployments.as_ref() {
        Some(d) => d.evm.clone(),
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

    // Save the updated deployments
    let updated = config::Deployments {
        evm: evm_deployment,
    };
    println!("Saving updated deployments...");
    config::save_deployments_to_path(&updated, &args.deployments)?;

    println!("Done!");

    Ok(())
}
