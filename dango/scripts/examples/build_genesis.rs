//! Write the genesis state used for tests to the CometBFT genesis file. Can be
//! used to spin up an actual network (e.g. using LocalDango) with an identical
//! environment as the tests.

use {
    anyhow::anyhow,
    chrono::{DateTime, Utc},
    clap::Parser,
    dango_genesis::{DexOption, GenesisCodes, GenesisOption, build_genesis},
    dango_testing::Preset,
    dango_types::{
        constants::{
            FIFTY, ONE, ONE_HUNDRED, ONE_HUNDREDTH, ONE_TENTH, TEN, btc, dango, eth, sol, usdc,
        },
        dex::{Geometric, PairParams, PairUpdate, PassiveLiquidity},
    },
    grug::{
        Bounded, Dec, Denom, Inner, Json, JsonDeExt, JsonSerExt, NonZero, NumberConst, Udec128,
        Uint128, btree_set,
    },
    grug_vm_rust::RustVm,
    std::{
        collections::BTreeSet,
        fs,
        path::{Path, PathBuf},
        str::FromStr,
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

    let (genesis_state, contracts, addresses) = build_genesis(
        RustVm::genesis_codes(),
        GenesisOption {
            dex: DexOption {
                pairs: vec![
                    PairUpdate {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: Denom::from_str("dex/pool/dango/usdc").unwrap(),
                            pool_type: PassiveLiquidity::Geometric(Geometric {
                                limit: 1,
                                spacing: Udec128::new_bps(1),
                                ratio: Bounded::new_unchecked(Dec::ONE),
                            }),
                            bucket_sizes: BTreeSet::new(), /* TODO: determine appropriate price buckets based on expected dango token price */
                            swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(1)),
                            min_order_size: Uint128::ZERO, /* TODO: for mainnet, a minimum of $10 is sensible */
                        },
                    },
                    PairUpdate {
                        base_denom: btc::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: Denom::from_str("dex/pool/btc/usdc").unwrap(),
                            pool_type: PassiveLiquidity::Geometric(Geometric {
                                limit: 1,
                                spacing: Udec128::new_bps(1),
                                ratio: Bounded::new_unchecked(Dec::ONE),
                            }),
                            bucket_sizes: btree_set! {
                                NonZero::new_unchecked(ONE_HUNDREDTH),
                                NonZero::new_unchecked(ONE_TENTH),
                                NonZero::new_unchecked(ONE),
                                NonZero::new_unchecked(TEN),
                                NonZero::new_unchecked(FIFTY),
                                NonZero::new_unchecked(ONE_HUNDRED),
                            },
                            swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(1)),
                            min_order_size: Uint128::ZERO,
                        },
                    },
                    PairUpdate {
                        base_denom: eth::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: Denom::from_str("dex/pool/eth/usdc").unwrap(),
                            pool_type: PassiveLiquidity::Geometric(Geometric {
                                limit: 1,
                                spacing: Udec128::new_bps(1),
                                ratio: Bounded::new_unchecked(Dec::ONE),
                            }),
                            bucket_sizes: btree_set! {
                                NonZero::new_unchecked(ONE_HUNDREDTH),
                                NonZero::new_unchecked(ONE_TENTH),
                                NonZero::new_unchecked(ONE),
                                NonZero::new_unchecked(TEN),
                                NonZero::new_unchecked(FIFTY),
                                NonZero::new_unchecked(ONE_HUNDRED),
                            },
                            swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(1)),
                            min_order_size: Uint128::ZERO,
                        },
                    },
                    PairUpdate {
                        base_denom: sol::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        params: PairParams {
                            lp_denom: Denom::from_str("dex/pool/sol/usdc").unwrap(),
                            pool_type: PassiveLiquidity::Geometric(Geometric {
                                limit: 1,
                                spacing: Udec128::new_bps(1),
                                ratio: Bounded::new_unchecked(Dec::ONE),
                            }),
                            bucket_sizes: btree_set! {
                                NonZero::new_unchecked(ONE_HUNDREDTH),
                                NonZero::new_unchecked(ONE_TENTH),
                                NonZero::new_unchecked(ONE),
                                NonZero::new_unchecked(TEN),
                            },
                            swap_fee_rate: Bounded::new_unchecked(Udec128::new_bps(1)),
                            min_order_size: Uint128::ZERO,
                        },
                    },
                ],
            },
            ..Preset::preset_test()
        },
    )?;

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
        )
        .map_err(|e| {
            anyhow!(
                "failed to update genesis file {}\nreason: {e}",
                path.display()
            )
        })?;
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
