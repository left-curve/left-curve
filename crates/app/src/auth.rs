use {
    crate::app::{ACCOUNTS, CODES, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{BlockInfo, Context, Storage, Tx},
    cw_vm::Instance,
    tracing::{debug, warn},
};

pub fn authenticate_tx<S: Storage + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> anyhow::Result<()> {
    match _authenticate_tx(store, block, tx) {
        Ok(()) => {
            // TODO: add txhash here?
            debug!(sender = tx.sender.to_string(), "tx authenticated");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "failed to authenticate tx");
            Err(err)
        },
    }
}

fn _authenticate_tx<S: Storage + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> anyhow::Result<()> {
    // load wasm code
    let account = ACCOUNTS.load(&store, &tx.sender)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store, &[CONTRACT_NAMESPACE, tx.sender.as_ref()]);
    let mut instance = Instance::build_from_code(substore, wasm_byte_code.as_ref())?;

    // call `before_tx` entry point
    let ctx = Context {
        block:    block.clone(),
        contract: tx.sender.clone(),
        sender:   None,
        simulate: Some(false),
    };
    let resp = instance.call_before_tx(&ctx, tx)?.into_std_result()?;

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(())
}
