use {
    clap::Parser,
    dango_hyperlane_deployment::{config, dango, setup},
    dotenvy::dotenv,
};

#[derive(Parser)]
#[command(name = "dango_set_ism_validator_set")]
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

    dango::set_ism_validator_set(&dango_client, &config, &mut dango_owner, evm_config).await
}
