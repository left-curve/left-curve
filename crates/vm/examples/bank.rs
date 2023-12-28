use {
    anyhow::anyhow,
    cw_bank::ExecuteMsg,
    cw_std::{MockStorage, Storage},
    cw_vm::{call_execute, db_read, db_remove, db_write, Host, InstanceBuilder},
    lazy_static::lazy_static,
    std::{env, path::PathBuf},
};

lazy_static! {
    static ref INITIAL_STATE: MockStorage = {
        let mut store = MockStorage::default();
        store.write(b"alice",   100u64.to_be_bytes().as_slice());
        store.write(b"bob",      50u64.to_be_bytes().as_slice());
        store.write(b"charlie", 123u64.to_be_bytes().as_slice());
        store
    };
}

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
    for (name_bytes, balance_bytes) in store.into_data() {
        let name = String::from_utf8(name_bytes)?;
        let balance = u64::from_be_bytes(balance_bytes.try_into()
            .map_err(|_| anyhow!("Failed to parse balance"))?);
        println!("name = {name}, balance = {balance}");
    }

    Ok(())
}

fn call_send<T>(host: &mut Host<T>, from: &str, to: &str, amount: u64) -> anyhow::Result<()> {
    println!("Sending... from: {from}, to: {to}, amount: {amount}");

    let res = call_execute(
        host,
        ExecuteMsg::Send {
            from: from.into(),
            to: to.into(),
            amount,
        },
    )?;

    println!("Contract response: {res:?}");

    Ok(())
}
