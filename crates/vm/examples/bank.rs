//! How to run this example:
//!
//! $ just optimize
//! $ cargo run -p cw-vm --example bank

use {
    cw_bank::{Balance, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw_std::{from_json, to_json, Addr, Context, Uint128},
    cw_vm::{
        db_next, db_read, db_remove, db_scan, db_write, debug, Host, InstanceBuilder, MockStorage,
    },
    std::{env, path::PathBuf},
};

// (address, denom, amount)
const BALANCES: [(Addr, &str, Uint128); 4] = [
    (Addr::mock(1), "uatom", Uint128::new(100)),
    (Addr::mock(1), "uosmo", Uint128::new(888)),
    (Addr::mock(2), "uatom", Uint128::new(50)),
    (Addr::mock(3), "uatom", Uint128::new(123)),
];

fn main() -> anyhow::Result<()> {
    // set tracing to TRACE level, so that we can see DB reads/writes logs
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    // create Wasm host instance
    let artifacts_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("../../artifacts");
    let wasm_file_path = {
        #[cfg(target_arch = "aarch64")]
        { artifacts_dir.join("cw_bank-aarch64.wasm") }
        #[cfg(not(target_arch = "aarch64"))]
        { artifacts_dir.join("cw_bank.wasm") }
    };
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file_path)?
        .with_storage(MockStorage::new())
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .with_host_function("db_scan", db_scan)?
        .with_host_function("db_next", db_next)?
        .with_host_function("debug", debug)?
        .finalize()?;
    let mut host = Host::new(&instance, &mut store);

    // deploy the contract
    instantiate(&mut host)?;

    // make some transfers
    send(&mut host, Addr::mock(1), Addr::mock(4), "uatom", 75)?;
    send(&mut host, Addr::mock(1), Addr::mock(5), "uosmo", 420)?;
    send(&mut host, Addr::mock(2), Addr::mock(3), "uatom", 50)?;
    send(&mut host, Addr::mock(3), Addr::mock(1), "uatom", 69)?;
    send(&mut host, Addr::mock(5), Addr::mock(6), "uosmo", 64)?;

    // query and print out the balances
    // should be:
    // 0x1: 94 uatom, 468 uosmo
    // 0x2: no balance, deleted from storage
    // 0x3: 104 uatom
    // 0x4: 75 uatom
    // 0x5: 356 uosmo
    // 0x6: 64 uosmo
    query_balances(&mut host)?;

    // if we need the host state for other purposes, we can consume the wasm
    // store here and take out the host state
    let _host_state = store.into_data();

    println!("✅ Done!");

    Ok(())
}

fn instantiate<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    println!("🤖 Instantiating contract");

    let mut initial_balances = vec![];
    for (address, denom, amount) in BALANCES {
        initial_balances.push(Balance {
            address,
            denom: denom.into(),
            amount,
        });
    }

    let res = host.call_instantiate(
        &mock_context(Some(Addr::mock(0))),
        to_json(&InstantiateMsg {
            initial_balances,
        })?,
    )?;

    println!("✅ Instantiation successful! res={}", serde_json::to_string(&res)?);

    Ok(())
}

fn send<T>(
    host:   &mut Host<T>,
    from:   Addr,
    to:     Addr,
    denom:  &str,
    amount: u128,
) -> anyhow::Result<()> {
    println!("🤖 Sending... from={from:?} to={to:?} denom={denom} amount={amount}");

    let res = host.call_execute(
        &mock_context(Some(from)),
        to_json(&ExecuteMsg::Send {
            to,
            denom: denom.into(),
            amount: Uint128::new(amount),
        })?,
    )?;

    println!("✅ Send successful! res={}", serde_json::to_string(&res)?);

    Ok(())
}

fn query_balances<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    println!("🤖 Querying balances");

    let res_bytes = host.call_query(
        &mock_context(None),
        to_json(&QueryMsg::Balances {
            start_after: None,
            limit:       None,
        })?,
    )?;

    let res: Vec<Balance> = from_json(res_bytes)?;

    println!("{}", serde_json::to_string_pretty(&res)?);

    Ok(())
}

// for this example, we don't use a context that resembles a real blockchain.
// see the example in cw-app instead.
fn mock_context(sender: Option<Addr>) -> Context {
    Context {
        block_height:    0,
        block_timestamp: 0,
        sender,
    }
}
