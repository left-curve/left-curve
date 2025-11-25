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
    dango_client::{Secp256k1, Secret, SingleSigner},
    dango_types::{
        config::AppConfig,
        gateway::{self, Origin, Remote},
    },
    dotenvy::dotenv,
    grug::{Addr, BroadcastClientExt, Coins, GasOption, Part, QueryClientExt, addr, btree_set},
    hex_literal::hex,
    indexer_client::HttpClient,
    tokio::time::sleep,
};

const DANGO_API_URL: &str = "https://api-pr-1414-ovh2.dango.zone/";

const DANGO_OWNER_ADDR: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");
const DANGO_OWNER_USERNAME: &str = "owner";
const DANGO_OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const CHAIN_ID: &str = "pr-1414";

const SUBDENOM: &str = "sepoliaETH";
const REMOTE_DOMAIN: u32 = dango_scripts::addresses::sepolia::WARP_DOMAIN;
const REMOTE_WARP_ADDRESS: Address =
    dango_scripts::addresses::sepolia::hyperlane_deployments::eth::WARP_ROUTE_PROXY;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let dango_client = HttpClient::new(DANGO_API_URL)?;
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    // Setup Dango owner
    let mut dango_owner = SingleSigner::new(
        DANGO_OWNER_USERNAME,
        DANGO_OWNER_ADDR,
        Secp256k1::from_bytes(DANGO_OWNER_PRIVATE_KEY)?,
    )?
    .with_query_nonce(&dango_client)
    .await?;

    // Set the route on the gateway
    println!("setting route on the gateway...");
    let outcome = dango_client
        .execute(
            &mut dango_owner,
            app_cfg.addresses.gateway,
            &gateway::ExecuteMsg::SetRoutes(btree_set! {
                (Origin::Remote(Part::new_unchecked(SUBDENOM)), app_cfg.addresses.warp, Remote::Warp {
                    domain: REMOTE_DOMAIN,
                    contract: REMOTE_WARP_ADDRESS.into_word().0.into()
                })
            }),
            Coins::new(),
            GasOption::Predefined {
                gas_limit: 1_000_000_u64,
            },
            CHAIN_ID,
        )
        .await?;

    println!("outcome: {:#?}", outcome);

    sleep(std::time::Duration::from_secs(1)).await;

    // Query the route on the gateway
    println!("querying the route on the gateway...");
    let denom = dango_client
        .query_wasm_smart(
            app_cfg.addresses.gateway,
            gateway::QueryRouteRequest {
                bridge: app_cfg.addresses.warp,
                remote: Remote::Warp {
                    domain: REMOTE_DOMAIN,
                    contract: REMOTE_WARP_ADDRESS.into_word().0.into(),
                },
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

    Ok(())
}
