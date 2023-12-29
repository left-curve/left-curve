use {
    cw_bank::{Balance, InstantiateMsg, ExecuteMsg, QueryMsg},
    cw_sdk::MockStorage,
    cw_vm::{call_execute, call_instantiate, call_query, db_read, db_remove, db_write, Host, InstanceBuilder},
    std::{env, path::PathBuf},
};

const BALANCES: [(&str, &str, u64); 4] = [
    ("alice",   "uatom", 100),
    ("alice",   "uosmo", 888),
    ("bob",     "uatom",  50),
    ("charlie", "uatom", 123),
];

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../../target/wasm32-unknown-unknown/debug/cw_bank.wasm");
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(MockStorage::new())
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
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
    println!("Balances after the transfers:");
    let query_res = call_query(&mut host, &QueryMsg::Balance {
        address: "alice".into(),
        denom:   "uatom".into(),
    })?;
    dbg!(query_res);

    // if we need the host state for other purposes, we can consume the wasm
    // store here and take out the host state
    let _host_state = store.into_data();

    Ok(())
}

fn instantiate<T>(host: &mut Host<T>) -> anyhow::Result<()> {
    println!("Instantiating contract...");

    let mut initial_balances = vec![];
    for (address, denom, amount) in BALANCES {
        initial_balances.push(Balance {
            address: address.into(),
            denom:   denom.into(),
            amount,
        });
    }
    let res = call_instantiate(host, &InstantiateMsg { initial_balances })?;

    println!("Contract response: {res:?}");

    Ok(())
}

fn send<T>(
    host:   &mut Host<T>,
    from:   &str,
    to:     &str,
    denom:  &str,
    amount: u64,
) -> anyhow::Result<()> {
    println!("Sending... from: {from}, to: {to}, denom: {denom}, amount: {amount}");

    let res = call_execute(host, &ExecuteMsg::Send {
        from:  from.into(),
        to:    to.into(),
        denom: denom.into(),
        amount,
    })?;

    println!("Contract response: {res:?}");

    Ok(())
}
