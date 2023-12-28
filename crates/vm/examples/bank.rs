use {
    cw_bank::ExecuteMsg,
    cw_sdk::{from_json, Map, MockStorage},
    cw_vm::{call_execute, db_read, db_remove, db_write, Host, InstanceBuilder},
    lazy_static::lazy_static,
    std::{env, path::PathBuf},
};

lazy_static! {
    static ref INITIAL_STATE: MockStorage = {
        let mut store = MockStorage::default();
        BALANCES.save(&mut store, &"alice".into(),   &100).unwrap();
        BALANCES.save(&mut store, &"bob".into(),     &50).unwrap();
        BALANCES.save(&mut store, &"charlie".into(), &123).unwrap();
        store
    };
}

const BALANCES: Map<String, u64> = Map::new("b");

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../../target/wasm32-unknown-unknown/debug/cw_bank.wasm");
    let (instance, mut store) = InstanceBuilder::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(INITIAL_STATE.clone())
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .finalize()?;
    let mut host = Host::new(&instance, &mut store);

    // make three transfers
    call_send(&mut host, "alice", "dave", 75)?;
    call_send(&mut host, "bob", "charlie", 50)?;
    call_send(&mut host, "charlie", "alice", 69)?;

    // end state:
    // ----------
    // alice:   100 - 75 + 69 = 94
    // bob:     50  - 50      = 0 (deleted from host state)
    // charlie: 123 + 50 - 69 = 104
    // dave:    0   + 75      = 75
    println!("Host state after aforementioned transfers:");
    // TODO: replace this with Map::range
    for (k, v) in store.into_data() {
        let name = String::from_utf8(k[3..].to_vec())?;
        let balance: u64 = from_json(&v)?;
        println!("name = {name}, balance = {balance}");
    }

    Ok(())
}

fn call_send<T>(host: &mut Host<T>, from: &str, to: &str, amount: u64) -> anyhow::Result<()> {
    println!("Sending... from: {from}, to: {to}, amount: {amount}");

    let res = call_execute(host, ExecuteMsg::Send {
        from: from.into(),
        to:   to.into(),
        amount,
    })?;

    println!("Contract response: {res:?}");

    Ok(())
}
