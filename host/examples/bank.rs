use {
    anyhow::anyhow,
    host::{Host, HostBuilder},
    std::{collections::BTreeMap, env, path::PathBuf},
    wasmi::{core::Trap, Caller},
};

// our host state is a generic key-value store.
//
// for this example, we interpret the keys as names of users (in UTF-8 encoding)
// and values as their bank balances (uint64 in big endian encoding).
type HostState = BTreeMap<Vec<u8>, Vec<u8>>;

// This is our initial host state before any calls
const INITIAL_STATE: &[(&str, u64)] = &[
    ("alice",   100),
    ("bob",     50),
    ("charlie", 123),
];

fn main() -> anyhow::Result<()> {
    let wasm_file = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
        .join("../target/wasm32-unknown-unknown/debug/bank.wasm");
    let data = INITIAL_STATE
        .into_iter()
        .map(|(name, balance)| (name.as_bytes().to_vec(), balance.to_be_bytes().to_vec()))
        .collect();
    let mut instance = HostBuilder::<HostState>::default()
        .with_wasm_file(wasm_file)?
        .with_host_state(data)
        .with_host_function("db_read", db_read)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .finalize()?;

    println!("alice sending 75 coins to dave...");
    call_send(&mut instance, "alice", "dave", 75)?;

    println!("bob sending 50 coins to charlie...");
    call_send(&mut instance, "bob", "charlie", 50)?;

    println!("charlie sending 69 coins to alice...");
    call_send(&mut instance, "charlie", "alice", 69)?;

    // end state:
    // ----------
    // alice:   100 - 75 + 69 = 94
    // bob:     50  - 50      = 0 (deleted from host state)
    // charlie: 123 + 50 - 69 = 104
    // dave:    0   + 75      = 75
    println!("Host state after aforementioned transfers:");
    for (name_bytes, balance_bytes) in instance.recycle() {
        let name = String::from_utf8(name_bytes)?;
        let balance = u64::from_be_bytes(balance_bytes.try_into()
            .map_err(|_| anyhow!("Failed to parse balance"))?);
        println!("name = {name}, balance = {balance}");
    }

    Ok(())
}

fn db_read<'a>(caller: Caller<'a, HostState>, key_ptr: u32) -> Result<u32, Trap> {
    let mut host = Host::build_ref(caller)?;
    let key = host.consume_region(key_ptr)?;

    // read the value from host state
    // if doesn't exist, we return a zero pointer
    let Some(value) = host.data().get(&key).cloned() else {
        return Ok(0);
    };

    // now we need to allocate a region in Wasm memory and put the value in
    let value_ptr = host.release_buffer(value)?;

    Ok(value_ptr)
}

fn db_write<'a>(
    caller: Caller<'a, HostState>,
    key_ptr:    u32,
    value_ptr:  u32,
) -> Result<(), Trap> {
    let mut host = Host::build_ref(caller)?;
    let key = host.consume_region(key_ptr)?;
    let value = host.consume_region(value_ptr)?;

    host.data_mut().insert(key, value);

    Ok(())
}

fn db_remove<'a>(caller: Caller<'a, HostState>, key_ptr: u32) -> Result<(), Trap> {
    let mut host = Host::build_ref(caller)?;
    let key = host.consume_region(key_ptr)?;

    host.data_mut().remove(&key);

    Ok(())
}

fn call_send(
    instance: &mut Host<HostState>,
    from: &str,
    to: &str,
    amount: u64,
) -> anyhow::Result<()> {
    // load sender into memory
    let from_ptr = instance.release_buffer(from.as_bytes().to_vec())?;

    // load receiver into memory
    let to_ptr = instance.release_buffer(to.as_bytes().to_vec())?;

    // call send function. this function has no return data
    instance.call("send", (from_ptr, to_ptr, amount))?;

    // no need to deallocate {from,to}_ptr, they were already freed in Wasm code
    // the send function doesn't have response data either, so we're done

    Ok(())
}
