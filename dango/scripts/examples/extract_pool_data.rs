use std::path::PathBuf;

use grug::addr;

use {dango_genesis::GenesisCodes, grug::JsonSerExt};

use {
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_db_memory_lite::MemDbLite,
    grug_vm_rust::RustVm,
};

const HEIGHT: u64 = 150721; // exclusive

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    RustVm::genesis_codes();

    let app = App::new(
        MemDbLite::recover(cwd.join(format!("db-{HEIGHT}.borsh")))?,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    let dex_addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

    let oracle_addr = addr!("fd3de90306e28197f277096fad988f38af1586b8");

    // Query app config
    let res = app.do_query_app(
        grug::Query::AppConfig(grug::QueryAppConfigRequest {}),
        HEIGHT,
        false,
    )?;
    println!("res: {}", res.to_json_string_pretty()?);

    // Query dex reserves
    let res = app.do_query_app(
        grug::Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: dex_addr,
            msg: dango_types::dex::QueryMsg::Reserve {
                base_denom: dango_types::constants::eth::DENOM.clone(),
                quote_denom: dango_types::constants::usdc::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;

    println!("res: {}", res.to_json_string_pretty()?);

    // Query oracle price of eth and usdc
    let res = app.do_query_app(
        grug::Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: oracle_addr,
            msg: dango_types::oracle::QueryMsg::Price {
                denom: dango_types::constants::eth::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;

    println!("res: {}", res.to_json_string_pretty()?);

    let res = app.do_query_app(
        grug::Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: oracle_addr,
            msg: dango_types::oracle::QueryMsg::Price {
                denom: dango_types::constants::usdc::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;

    println!("res: {}", res.to_json_string_pretty()?);

    // Query pool params for ETHUSDC
    let res = app.do_query_app(
        grug::Query::WasmSmart(grug::QueryWasmSmartRequest {
            contract: dex_addr,
            msg: dango_types::dex::QueryMsg::Pair {
                base_denom: dango_types::constants::eth::DENOM.clone(),
                quote_denom: dango_types::constants::usdc::DENOM.clone(),
            }
            .to_json_value()?,
        }),
        HEIGHT,
        false,
    )?;

    println!("res: {}", res.to_json_string_pretty()?);

    Ok(())
}
