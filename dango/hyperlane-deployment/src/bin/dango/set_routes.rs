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
    dango_hyperlane_deployment::setup,
    dango_types::{config::AppConfig, gateway},
    dotenvy::dotenv,
    grug::{BroadcastClientExt, Coins, GasOption, QueryClientExt, btree_set},
    tokio::time::sleep,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let (dango_client, mut dango_owner, config) = setup::setup_dango().await?;

    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    for route in config.routes {
        // Set the route on the gateway
        println!(
            "setting route for origin: {:?}, remote: {:?}",
            route.origin, route.remote
        );
        dango_client
            .execute(
                &mut dango_owner,
                app_cfg.addresses.gateway,
                &gateway::ExecuteMsg::SetRoutes(btree_set! {
                    (route.origin, app_cfg.addresses.warp, route.remote)
                }),
                Coins::new(),
                GasOption::Predefined {
                    gas_limit: 1_000_000_u64,
                },
                config.dango_chain_id.as_str(),
            )
            .await?;

        sleep(std::time::Duration::from_millis(500)).await;

        // Query the route on the gateway
        println!("querying the route on the gateway...");
        let denom = dango_client
            .query_wasm_smart(
                app_cfg.addresses.gateway,
                gateway::QueryRouteRequest {
                    bridge: app_cfg.addresses.warp,
                    remote: route.remote,
                },
                None,
            )
            .await?;

        if let Some(denom) = denom {
            println!(
                "Successfully set the route. Denom in use on Dango: {:#?}",
                denom
            );
        } else {
            return Err(anyhow::anyhow!("Failed to set the route"));
        }
    }

    Ok(())
}
