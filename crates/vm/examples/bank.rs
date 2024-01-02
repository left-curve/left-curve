use {
    cw_bank::{Balance, ExecuteMsg, InstantiateMsg, QueryMsg},
    cw_std::{from_json, Addr, MockStorage, Uint128},
    cw_vm::{db_next, db_read, db_remove, db_scan, db_write, debug, Host, InstanceBuilder},
    std::{env, path::PathBuf},
    tracing::info,
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
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../../target/wasm32-unknown-unknown/debug/cw_bank.wasm");
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
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

    Ok(())
}

fn instantiate<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    info!("instantiating contract");

    let mut initial_balances = vec![];
    for (address, denom, amount) in BALANCES {
        initial_balances.push(Balance {
            address,
            denom: denom.into(),
            amount,
        });
    }
    let res = host.call_instantiate(&InstantiateMsg { initial_balances })?;

    info!(?res, "instantiation successful");

    Ok(())
}

fn send<T>(
    host:   &mut Host<T>,
    from:   Addr,
    to:     Addr,
    denom:  &str,
    amount: u128,
) -> anyhow::Result<()> {
    info!(?from, ?to, denom, amount, "sending");

    let res = host.call_execute(&ExecuteMsg::Send {
        from,
        to,
        denom:  denom.into(),
        amount: Uint128::new(amount),
    })?;

    info!(?res, "send successful");

    Ok(())
}

fn query_balances<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    info!("querying balances");

    let res_bytes = host.call_query(&QueryMsg::Balances {
        start_after: None,
        limit:       None,
    })?
    .into_result()?;

    let res: Vec<Balance> = from_json(&res_bytes)?;

    println!("{}", serde_json_wasm::to_string(&res)?);

    Ok(())
}
