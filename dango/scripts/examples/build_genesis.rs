//! Write the genesis state used for tests to the CometBFT genesis file. Can be
//! used to spin up an actual network (e.g. using LocalDango) with an identical
//! environment as the tests.

use {
    anyhow::anyhow,
    chrono::{DateTime, Utc},
    clap::Parser,
    dango_genesis::{GenesisCodes, GenesisOption, build_genesis},
    dango_testing::Preset,
    grug::{Inner, Json, JsonDeExt, JsonSerExt},
    grug_vm_rust::RustVm,
    std::{
        fs,
        path::{Path, PathBuf},
    },
};

#[derive(Parser)]
struct Cli {
    /// Paths to the CometBFT genesis files
    #[arg(num_args(1..))]
    paths: Vec<PathBuf>,

    /// Optionally update the chain ID (e.g. "dev-1")
    #[arg(long)]
    chain_id: Option<String>,

    /// Optionally update the genesis time (e.g. "2025-08-21T14:00:00Z")
    #[arg(long)]
    genesis_time: Option<DateTime<Utc>>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let (genesis_state, contracts, addresses) =
        build_genesis(RustVm::genesis_codes(), GenesisOption::preset_test())?;

    println!("genesis_state = {}", genesis_state.to_json_string_pretty()?);
    println!("\ncontracts = {}", contracts.to_json_string_pretty()?);
    println!("\naddresses = {}\n", addresses.to_json_string_pretty()?);

    let genesis_state = genesis_state.to_json_value()?;
    let chain_id = cli.chain_id.map(|id| id.to_json_value()).transpose()?;
    let genesis_time = cli.genesis_time.map(|t| t.to_json_value()).transpose()?;

    for path in cli.paths {
        update_genesis_file(
            &path,
            genesis_state.clone(),
            chain_id.clone(),
            genesis_time.clone(),
        )?;
    }

    Ok(())
}

fn update_genesis_file(
    path: &Path,
    genesis_state: Json,
    chain_id: Option<Json>,
    genesis_time: Option<Json>,
) -> anyhow::Result<()> {
    let mut cometbft_genesis = fs::read(path)?.deserialize_json::<Json>()?;

    let map = cometbft_genesis.as_object_mut().ok_or_else(|| {
        anyhow!(
            "cometbft genesis file `{}` isn't a json object",
            path.display()
        )
    })?;

    map.insert("app_state".to_string(), genesis_state.into_inner());

    if let Some(chain_id) = chain_id {
        map.insert("chain_id".to_string(), chain_id.into_inner());
    }

    if let Some(genesis_time) = genesis_time {
        map.insert("genesis_time".to_string(), genesis_time.into_inner());
    }

    let mut output = cometbft_genesis.to_json_string_pretty()?;
    output.push('\n'); // add a newline to end of file: https://stackoverflow.com/questions/729692/

    fs::write(path, output)?;
    println!("updated genesis file written to: {}", path.display());

    Ok(())
}
