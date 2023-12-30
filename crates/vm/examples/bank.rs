use {
    cw_bank::{Balance, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw_sdk::from_json,
    cw_vm::{
        call_execute, call_instantiate, call_query, db_next, db_read, db_remove, db_scan, db_write,
        debug, Host, InstanceBuilder, MockHostState,
    },
    std::{env, path::PathBuf},
    tracing::info,
};

const BALANCES: [(&str, &str, u64); 4] = [
    ("alice",   "uatom", 100),
    ("alice",   "uosmo", 888),
    ("bob",     "uatom",  50),
    ("charlie", "uatom", 123),
];

fn main() -> anyhow::Result<()> {
    // set tracing to TRACE level, so that we can see logs of DB reads/writes
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    // incrase Wasm host instance
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../../target/wasm32-unknown-unknown/debug/cw_bank.wasm");
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(MockHostState::new())
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .with_host_function("db_scan", db_scan)?
        .with_host_function("db_next", db_next)?
        .with_host_function("debug", debug)?
        .finalize()?;
    let mut host = Host::new(&instance, &mut store);

    // instantiate contract
    instantiate(&mut host)?;

    // make three transfers
    send(&mut host, "alice", "dave", "uatom", 75)?;
    send(&mut host, "bob", "charlie", "uatom", 50)?;
    send(&mut host, "charlie", "alice", "uatom", 69)?;

    // end state:
    // ----------
    // alice:   100 - 75 + 69 = 94
    // bob:     50  - 50      = 0 (deleted from host state)
    // charlie: 123 + 50 - 69 = 104
    // dave:    0   + 75      = 75
    query_balances(&mut host)?;

    // if we need the host state for other purposes, we can consume the wasm
    // store here and take out the host state
    let _host_state = store.into_data();

    Ok(())
}

fn instantiate<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    info!("instantiating contract");

    let mut initial_balances = vec![];
    for (address, denom, amount) in BALANCES {
        initial_balances.push(Balance {
            address: address.into(),
            denom:   denom.into(),
            amount,
        });
    }
    let res = call_instantiate(host, &InstantiateMsg { initial_balances })?;

    info!(?res, "instantiation successful");

    Ok(())
}

fn send<T>(
    host:   &mut Host<T>,
    from:   &str,
    to:     &str,
    denom:  &str,
    amount: u64,
) -> anyhow::Result<()> {
    info!(from, to, denom, amount, "sending");

    let res = call_execute(host, &ExecuteMsg::Send {
        from:  from.into(),
        to:    to.into(),
        denom: denom.into(),
        amount,
    })?;

    info!(?res, "send successful");

    Ok(())
}

fn query_balances<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    info!("querying balances");

    let res_bytes = call_query(host, &QueryMsg::Balances {
        start_after: None,
        limit:       None,
    })?
    .into_result()?;

    let res: Vec<Balance> = from_json(&res_bytes)?;

    info!(?res, "query successful");

    Ok(())
}
