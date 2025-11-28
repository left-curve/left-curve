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
    alloy::primitives::Address,
    dango_client::SingleSigner,
    dango_hyperlane_deployment::{addresses::sepolia::hyperlane_deployments::eth, setup},
    dango_types::{
        config::AppConfig,
        gateway::{self, Remote},
    },
    dotenvy::dotenv,
    grug::{BroadcastClientExt, Coins, GasOption, QueryClientExt, btree_set},
    tokio::time::sleep,
};

const REMOTE_WARP_CONTRACT: Address = eth::WARP_ROUTE_PROXY;
const WARP_AMOUNT: u64 = 100;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // dotenv()?;

    // let (dango_client, mut dango_owner, config) = setup::setup_dango().await?;

    // let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    // let user5 = setup::get_user5(&dango_client).await?;

    // // Query the denom from the gateway
    // let denom = dango_client
    //     .query_wasm_smart(
    //         app_cfg.addresses.gateway,
    //         gateway::QueryRouteRequest {
    //             bridge: app_cfg.addresses.warp,
    //             remote: Remote::Warp {
    //                 domain: 11155111,
    //                 contract: REMOTE_WARP_CONTRACT.into(),
    //             },
    //         },
    //         None,
    //     )
    //     .await?;

    // // Warp 100 sepoliaETH to remote
    // let tx_hash = dango_client
    //     .execute(
    //         &mut user5,
    //         app_cfg.addresses.warp,
    //         &gateway::ExecuteMsg::TransferRemote {
    //             remote: Remote::Warp {
    //                 domain: 11155111,
    //                 contract: REMOTE_WARP_CONTRACT.into(),
    //             },
    //             recipient: user5.address(),
    //         },
    //         Coins::new().insert(Coin {}),
    //     )
    //     .await?;

    Ok(())
}
