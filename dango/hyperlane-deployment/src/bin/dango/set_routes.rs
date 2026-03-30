//! This script sets warp routes on the Dango gateway.

use {
    alloy::primitives::{Address, address},
    clap::Parser,
    dango_hyperlane_deployment::{config, dango::set_warp_routes, setup},
    dotenvy::dotenv,
    std::collections::BTreeSet,
};

const ROUTES: &[(&str, Address)] = &[(
    "sepoliaETH",
    address!("0x613942eff27c6886bb2a33a172cdaf03a009e601"),
)];

#[derive(Parser)]
#[command(name = "dango_set_routes")]
struct Args {
    #[arg(long)]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;

    let args = Args::parse();

    let config = config::load_config_from_path(&args.config)?;
    let evm_config = &config.evm;

    let (dango_client, mut dango_owner) = setup::setup_dango(&config.dango).await?;

    set_warp_routes(
        &dango_client,
        &mut dango_owner,
        evm_config.hyperlane_domain,
        ROUTES
            .iter()
            .map(|(symbol, address)| (symbol.to_string(), *address))
            .collect::<BTreeSet<_>>(),
    )
    .await
}
