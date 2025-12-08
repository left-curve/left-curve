use {
    clap::Parser,
    dango_hyperlane_deployment::{
        config,
        dango::{self, set_warp_routes},
        evm::{deploy_proxy_admin, deploy_warp_route_and_update_deployment, get_or_deploy_ism},
        setup,
    },
    dotenvy::dotenv,
    std::collections::BTreeSet,
};

#[derive(Parser)]
#[command(name = "evm_deploy")]
#[command(about = "Deploys Hyperlane contracts on an EVM network")]
struct Args {
    /// The EVM network name (e.g., "sepolia")
    #[arg(long)]
    network: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let config = config::load_config()?;
    let mut deployments = config::load_deployments()?;

    let (dango_client, mut dango_owner) = setup::setup_dango(&config.dango).await?;

    let evm_config = config
        .evm
        .get(&args.network)
        .ok_or_else(|| anyhow::anyhow!("EVM network '{}' not found in config", args.network))?;

    let (provider, owner) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

    let mut evm_deployment = match deployments.evm.get(&args.network) {
        Some(deployment) => deployment.clone(),
        None => {
            let proxy_admin_address = deploy_proxy_admin(&provider).await?;
            config::EVMDeployment {
                proxy_admin_address,
                warp_routes: vec![],
            }
        },
    };

    let ism = evm_config.ism.clone();

    // Deploy the ISM
    let ism_address = get_or_deploy_ism(&provider, &evm_config.hyperlane_deployments, ism).await?;

    // Deploy the warp routes
    for warp_route in evm_config.warp_routes.iter() {
        deploy_warp_route_and_update_deployment(
            &provider,
            &dango_client,
            warp_route,
            owner,
            Some(ism_address),
            evm_config,
            &mut evm_deployment,
        )
        .await?;
    }

    // Update the deployments struct with the new EVM deployment adn save to file
    deployments
        .evm
        .insert(args.network.clone(), evm_deployment.clone());
    config::save_deployments(&deployments)?;

    // Set the routes on the Dango gateway
    let routes = evm_deployment
        .warp_routes
        .iter()
        .map(|(_, warp_route_deployment)| {
            (
                warp_route_deployment.symbol.clone(),
                warp_route_deployment.proxy_address,
            )
        })
        .collect::<BTreeSet<_>>();
    set_warp_routes(
        &dango_client,
        &config.dango,
        &mut dango_owner,
        evm_config.hyperlane_domain,
        routes,
    )
    .await?;

    // Set the validator set on the Dango gateway
    dango::set_ism_validator_set(&dango_client, &config, &mut dango_owner, evm_config).await?;

    println!("Done!");

    Ok(())
}
