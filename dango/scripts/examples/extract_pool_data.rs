use {
    dango_genesis::GenesisCodes,
    dango_types::{
        constants::{eth, usdc},
        dex, oracle,
    },
    grug::{Addr, JsonSerExt, Query, addr},
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_commitment_simple::Simple,
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    std::path::PathBuf,
};

const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

const ORACLE: Addr = addr!("fd3de90306e28197f277096fad988f38af1586b8");

const HEIGHT: u64 = 150721;

fn main() -> anyhow::Result<()> {
    let _codes = RustVm::genesis_codes();

    let snapshot = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(format!("db-{HEIGHT}.borsh"));

    let app = App::new(
        MemDb::<Simple>::recover(snapshot)?,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    // Query app config.
    let res = app.do_query_app(
        Query::AppConfig(grug::QueryAppConfigRequest {}),
        HEIGHT,
        false,
    )?;
    println!("app config: {}", res.to_json_string_pretty()?);

    // Query the oracle price of ETH.
    let res = app.do_query_app(
        Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: ORACLE,
            msg: oracle::QueryMsg::Price {
                denom: eth::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;
    println!("ETH price: {}", res.to_json_string_pretty()?);

    // Query the oracle price of USDC.
    let res = app.do_query_app(
        Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: ORACLE,
            msg: oracle::QueryMsg::Price {
                denom: usdc::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;
    println!("USDC price: {}", res.to_json_string_pretty()?);

    // Query the params of ETH-USDC pool.
    let res = app.do_query_app(
        Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: DEX,
            msg: dex::QueryMsg::Pair {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;
    println!("ETH-USDC pool params: {}", res.to_json_string_pretty()?);

    // Query the reserve of ETH-USDC pool.
    let res = app.do_query_app(
        Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: DEX,
            msg: dex::QueryMsg::Reserve {
                base_denom: eth::DENOM.clone(),
                quote_denom: usdc::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;
    println!("ETH-USDC pool reserve: {}", res.to_json_string_pretty()?);

    Ok(())
}
