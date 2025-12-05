use {
    crate::{
        config::{
            EVMDeployment, EVMWarpRouteDeployment,
            evm::{EVMConfig, HyperlaneDeployments, Ism, WarpRoute, WarpRouteType},
        },
        contract_bindings::{
            hyp_erc20_collateral::HypERC20Collateral,
            hyp_native::HypNative,
            ism::StaticMessageIdMultisigIsmFactory,
            proxy::{ProxyAdmin, TransparentUpgradeableProxy},
        },
    },
    alloy::{
        network::TransactionBuilder,
        primitives::{Address, FixedBytes, U256},
        providers::Provider,
        rpc::types::TransactionRequest,
        sol_types::SolCall,
    },
    dango_types::config::AppConfig,
    grug::{Inner, QueryClientExt},
    indexer_client::HttpClient,
};

pub mod utils;

pub async fn enroll_dango_domain(
    provider: &impl Provider,
    dango_client: &HttpClient,
    warp_proxy_address: Address,
) -> anyhow::Result<()> {
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    // Query the warp contract address from the app config and pad it to 32 bytes.
    let warp_contract_address = app_cfg.addresses.warp;
    let warp_contract_address_fixed =
        FixedBytes::<32>::left_padding_from(warp_contract_address.inner());

    // Query the mailbox config to get the dango domain
    let mailbox_config = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.mailbox,
            hyperlane_types::mailbox::QueryConfigRequest {},
            None,
        )
        .await?;
    let dango_domain = mailbox_config.local_domain;

    // Setup contract wrapper
    let hwr_proxy = TransparentUpgradeableProxy::new(warp_proxy_address, &provider);

    println!("Enrolling dango domain in router on Sepolia...");
    let tx_hash = provider
        .send_transaction(
            TransactionRequest::default()
                .with_to(*hwr_proxy.address())
                .with_call(&HypNative::enrollRemoteRouterCall {
                    _domain: dango_domain,
                    _router: warp_contract_address_fixed,
                }),
        )
        .await?
        .watch()
        .await?;
    println!("Done! tx hash: {tx_hash}");

    Ok(())
}

pub async fn deploy_proxy_admin(provider: &impl Provider) -> anyhow::Result<Address> {
    println!("Deploying ProxyAdmin contract...");
    let admin = ProxyAdmin::deploy(&provider).await?;
    println!("Done! ProxyAdmin address: {}", admin.address());
    Ok(*admin.address())
}

/// Deploys a new warp route contract according to the warp route type.
///
/// # Arguments
///
/// * `provider` - The provider to use for the deployment.
/// * `hyperlane_deployments` - The hyperlane deployments to use for the deployment.
/// * `warp_route` - The warp route to deploy.
/// * `proxy_admin_address` - The address of the proxy admin to use for the deployment.
/// * `ism` - The address of the ISM to use for the deployment.
/// * `owner` - The address of the owner to use for the deployment.
///
/// # Returns
///
/// * `(hyperlane_warp_route_address, proxy_address)` - The address of the deployed warp route and proxy.
///
/// # Errors
///
/// * `anyhow::Error` - If the deployment fails.
pub async fn deploy_warp_route(
    provider: &impl Provider,
    hyperlane_deployments: &HyperlaneDeployments,
    warp_route: &WarpRoute,
    proxy_admin_address: Address,
    ism: Option<Address>,
    owner: Address,
) -> anyhow::Result<(Address, Address)> {
    // Deploy new ProxyAdmin contract if not provided.
    let proxy_admin = ProxyAdmin::new(proxy_admin_address, &provider);

    // Deploy the warp route contract
    let hyperlane_warp_route = match warp_route.warp_route_type {
        WarpRouteType::ERC20Collateral(erc20_address) => {
            println!("Deploying HypERC20Collateral contract for {erc20_address}");
            let hyperlane_warp_route = HypERC20Collateral::deploy(
                &provider,
                erc20_address,
                U256::ONE,
                hyperlane_deployments.mailbox,
            )
            .await?;
            println!("Done! HWR address: {}", hyperlane_warp_route.address());
            *hyperlane_warp_route.address()
        },
        WarpRouteType::Native => {
            println!("Deploying HypNative contract...");
            let hyperlane_warp_route =
                HypNative::deploy(&provider, U256::ONE, hyperlane_deployments.mailbox).await?;
            println!("Done! HWR address: {}", hyperlane_warp_route.address());
            *hyperlane_warp_route.address()
        },
    };

    // Deploy the proxy contract
    println!("Deploying proxy contract...");
    let proxy = TransparentUpgradeableProxy::deploy(
        &provider,
        hyperlane_warp_route,
        *proxy_admin.address(),
        HypERC20Collateral::initializeCall {
            _hook: Address::ZERO,                                    // use mailbox default
            _interchainSecurityModule: ism.unwrap_or(Address::ZERO), // use mailbox default
            _owner: owner,
        }
        .abi_encode()
        .into(),
    )
    .await?;
    println!("Done! Proxy address: {}", proxy.address());

    Ok((hyperlane_warp_route, *proxy.address()))
}

/// Deploys a new warp route if it is not already deployed and updates the deployment with the new warp route.
pub async fn deploy_warp_route_and_update_deployment(
    provider: &impl Provider,
    warp_route: &WarpRoute,
    owner: Address,
    ism: Option<Address>,
    evm_config: &EVMConfig,
    deployment: &mut EVMDeployment,
) -> anyhow::Result<()> {
    // Return early if the warp route is already deployed
    if deployment
        .warp_routes
        .iter()
        .any(|(warp_route_type, _)| *warp_route_type == warp_route.warp_route_type)
    {
        println!(
            "Warp route {:?} for {} already deployed. Skipping...",
            warp_route.warp_route_type, warp_route.symbol
        );
        return Ok(());
    }

    println!(
        "Deploying warp route {:?} for {}...",
        warp_route.warp_route_type, warp_route.symbol
    );
    let (warp_route_address, proxy_address) = deploy_warp_route(
        &provider,
        &evm_config.hyperlane_deployments,
        warp_route,
        deployment.proxy_admin_address,
        ism,
        owner,
    )
    .await?;

    // Update the deployment with the new warp route
    deployment.warp_routes.push(
        (warp_route.warp_route_type.clone(), EVMWarpRouteDeployment {
            address: warp_route_address,
            proxy_address,
            symbol: warp_route.symbol.clone(),
        }),
    );

    Ok(())
}

/// Sets the ISM address on the warp route.
pub async fn set_ism_on_warp_route(
    provider: &impl Provider,
    warp_route_address: Address,
    ism_address: Address,
) -> anyhow::Result<()> {
    let warp_route = HypNative::new(warp_route_address, &provider);

    println!("Setting ISM on warp route {warp_route_address} to {ism_address}...");
    let tx = warp_route
        .setInterchainSecurityModule(ism_address)
        .send()
        .await?
        .watch()
        .await?;
    println!("Done! Tx hash: {tx}");

    Ok(())
}

/// Deploys a new ISM using the hyperlane factory. If the ISM is already deployed, it will
/// skip the deployment and return the address of the existing ISM.
pub async fn get_or_deploy_ism(
    provider: &impl Provider,
    hyperlane_deployments: &HyperlaneDeployments,
    ism: Ism,
) -> anyhow::Result<Address> {
    match ism {
        Ism::StaticMessageIdMultisigIsm {
            validators,
            threshold,
        } => {
            let factory = StaticMessageIdMultisigIsmFactory::new(
                hyperlane_deployments.static_message_id_multisig_ism_factory,
                &provider,
            );

            // Get the address of the ISM
            println!(
                "Querying the factory for the ISM address for validators {validators:?} and threshold {threshold}..."
            );
            let ism_address = factory
                .getAddress(validators.clone(), threshold)
                .call()
                .await?;
            println!("ISM address: {ism_address}");

            // Check if the ISM is already deployed
            let is_deployed = utils::is_contract(&provider, ism_address).await?;
            if !is_deployed {
                println!("ISM is not yet deployed. Deploying...");
                let tx = factory
                    .deploy_call(validators, threshold)
                    .send()
                    .await?
                    .watch()
                    .await?;
                println!("Done! Tx hash: {tx}");
            } else {
                println!("ISM is already deployed. Skipping deployment...");
            }

            Ok(ism_address)
        },
    }
}
