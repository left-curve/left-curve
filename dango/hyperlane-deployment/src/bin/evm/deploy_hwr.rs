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
        contract_bindings::proxy::ProxyAdmin,
        evm::{deploy_proxy_admin, deploy_warp_route, get_or_deploy_ism},
        setup,
    },
    dotenvy::dotenv,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let mut config = config::load_config()?;

    // Separate block to avoid borrowing issues with the config
    {
        let evm_config = config.evm.get_mut("sepolia").unwrap();
        let (provider, owner) = setup::evm::setup_ethereum_provider(&evm_config.infura_rpc_url)?;

        let ism = get_or_deploy_ism(
            &provider,
            &evm_config.hyperlane_deployments,
            evm_config.ism.clone(),
        )
        .await?;

        match evm_config.proxy_admin_address {
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

        deploy_warp_route(
            &provider,
            &evm_config.hyperlane_deployments,
            &evm_config.warp_routes[0],
            evm_config.proxy_admin_address.unwrap(),
            Some(ism),
            owner,
        )
        .await?;
    }

    // Save the config
    println!("Saving updated config...");
    config::save_config(&config)?;

    println!("Done!");

    Ok(())
}
