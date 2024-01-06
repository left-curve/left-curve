use {
    crate::PrefixStore,
    cw_std::{Addr, Storage},
    cw_vm::{db_read, db_write, db_remove, db_scan, db_next, debug, InstanceBuilder},
    wasmi::{Instance, Store},
};

pub(crate) fn must_build_wasm_instance<S: Storage + 'static>(
    store:  S,
    prefix: &[u8],
    addr:   &Addr,
    wasm:   impl AsRef<[u8]>,
) -> (Instance, Store<PrefixStore<S>>) {
    build_wasm_instance(store, prefix, addr, wasm)
        .unwrap_or_else(|err| panic!("Fatal error! Failed to build wasm instance: {err}"))
}

fn build_wasm_instance<S: Storage + 'static>(
    store:  S,
    prefix: &[u8],
    addr:   &Addr,
    wasm:   impl AsRef<[u8]>,
) -> anyhow::Result<(Instance, Store<PrefixStore<S>>)> {
    InstanceBuilder::default()
        .with_wasm_bytes(wasm)?
        .with_storage(PrefixStore::new(store, &[prefix, addr.as_ref()]))
        .with_host_function("db_read", db_read)?
        .with_host_function("db_scan", db_scan)?
        .with_host_function("db_next", db_next)?
        .with_host_function("db_write", db_write)?
        .with_host_function("db_remove", db_remove)?
        .with_host_function("debug", debug)?
        .finalize()
}
