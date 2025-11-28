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
    alloy::primitives::{Address, address},
    dango_hyperlane_deployment::{config, dango::set_warp_routes, setup},
    dango_types::config::AppConfig,
    dotenvy::dotenv,
    grug::{QueryClientExt, btree_set},
    std::collections::BTreeSet,
};

const ROUTES: BTreeSet<(String, Address)> = btree_set! {
    ("sepoliaETH", address!("0x613942eff27c6886bb2a33a172cdaf03a009e601")),
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let config = config::load_config()?;
    let evm_config = config.evm.get("sepolia").unwrap();

    let (dango_client, mut dango_owner) = setup::setup_dango(&config.dango).await?;

    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    set_warp_routes(
        &dango_client,
        &config.dango,
        &mut dango_owner,
        evm_config.hyperlane_domain,
        ROUTES,
    )
    .await?;
    Ok(())
}
