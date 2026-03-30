use {
    clap::Parser,
    dango_hyperlane_deployment::{
        config,
        dango::{self, set_warp_routes},
        evm::{
            deploy_proxy_admin, deploy_warp_route_and_update_deployment, get_or_deploy_ism,
            transfer_proxy_owner_ownership,
        },
        setup,
    },
    dotenvy::dotenv,
    std::collections::BTreeSet,
};

#[derive(Parser)]
#[command(name = "evm_deploy")]
#[command(about = "Deploys Hyperlane contracts on an EVM network")]
struct Args {
    /// Path to the config file
    #[arg(long)]
    config: String,
    /// Path to the deployments file
    #[arg(long)]
    deployments: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let config = config::load_config_from_path(&args.config)?;
    let deployments = config::load_deployments_from_path(&args.deployments).ok();

    let (dango_client, mut dango_owner) = setup::setup_dango(&config.dango).await?;

    let evm_config = &config.evm;
    println!(
        "Deploying for EVM chain '{}' (domain: {})",
        evm_config.chain_name, evm_config.hyperlane_domain
    );

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

    // Save deployments
    let updated = config::Deployments {
        evm: evm_deployment.clone(),
    };
    config::save_deployments_to_path(&updated, &args.deployments)?;

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
        &mut dango_owner,
        evm_config.hyperlane_domain,
        routes,
    )
    .await?;

    // Transfer ProxyAdmin ownership to multi-sig if configured
    if let Some(multi_sig_address) = evm_config.multi_sig_address {
        transfer_proxy_owner_ownership(
            &provider,
            evm_deployment.proxy_admin_address,
            multi_sig_address,
        )
        .await?;
    } else {
        println!(
            "No multi-sig address configured for chain '{}'. Skipping ProxyAdmin ownership transfer...",
            evm_config.chain_name
        );
    }

    // Set the validator set on the Dango gateway
    dango::set_ism_validator_set(&dango_client, &config, &mut dango_owner, evm_config).await?;

    println!(
        "Deployed Hyperlane contracts on '{}'. Done!",
        evm_config.chain_name
    );

    Ok(())
}
