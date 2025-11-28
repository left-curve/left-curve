use {
    clap::Parser,
    dango_hyperlane_deployment::{
        config,
        contract_bindings::proxy::ProxyAdmin,
        dango::set_warp_routes,
        evm::{deploy_proxy_admin, deploy_warp_route, get_or_deploy_ism},
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

    let mut config = config::load_config()?;

    // Separate block to avoid borrowing issues with the config
    {
        let evm_config = config
            .evm
            .get_mut(&args.network)
            .ok_or_else(|| anyhow::anyhow!("EVM network '{}' not found in config", args.network))?;

        let (dango_client, mut dango_owner) = setup::setup_dango(&config.dango).await?;

        let (provider, owner) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

        let ism = evm_config.ism.clone();

        // Deploy the ProxyAdmin if it is not provided and update the config
        let proxy_admin = match evm_config.proxy_admin_address {
            Some(proxy_admin_address) => {
                println!("Using provided ProxyAdmin contract at {proxy_admin_address}");
                ProxyAdmin::new(proxy_admin_address, &provider)
            },
            None => {
                let proxy_admin_address = deploy_proxy_admin(&provider).await?;
                evm_config.proxy_admin_address = Some(proxy_admin_address);
                ProxyAdmin::new(proxy_admin_address, &provider)
            },
        };

        // Deploy the ISM
        let ism_address =
            get_or_deploy_ism(&provider, &evm_config.hyperlane_deployments, ism).await?;

        // Deploy the warp routes
        for warp_route in evm_config.warp_routes.iter_mut() {
            if warp_route.address.is_none() != warp_route.proxy_address.is_none() {
                return Err(anyhow::anyhow!(
                    "warp_route.address and warp_route.proxy_address must be either both set or both unset"
                ));
            }

            // If the warp route is not deployed, deploy it
            if warp_route.address.is_none() {
                println!(
                    "Undeployed warp route for {} on {} found.",
                    warp_route.symbol, evm_config.hyperlane_domain
                );
                println!("Deploying...");
                let (hyperlane_warp_route_address, proxy_address) = deploy_warp_route(
                    &provider,
                    &evm_config.hyperlane_deployments,
                    warp_route,
                    *proxy_admin.address(),
                    Some(ism_address),
                    owner,
                )
                .await?;
                println!("Warp route successfully deployed.");

                // Update the warp route with the deployed address and proxy address
                warp_route.address = Some(hyperlane_warp_route_address);
                warp_route.proxy_address = Some(proxy_address);
            } else {
                println!(
                    "Skipping warp route for {} on {} already which is already deployed at {:#?}.",
                    warp_route.symbol,
                    evm_config.hyperlane_domain,
                    warp_route.address.unwrap()
                );
            }
        }

        // Set the route on the Dango gateway
        let routes = evm_config
            .warp_routes
            .iter()
            .map(|warp_route| (warp_route.symbol.clone(), warp_route.proxy_address.unwrap()))
            .collect::<BTreeSet<_>>();
        set_warp_routes(
            &dango_client,
            &config.dango,
            &mut dango_owner,
            evm_config.hyperlane_domain,
            routes,
        )
        .await?;
    }

    // Save the config
    println!("Saving updated config...");
    config::save_config(&config)?;

    println!("Done!");

    Ok(())
}
