use {
    crate::{
        new_before_tx_event, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONTRACT_NAMESPACE,
    },
    cw_db::PrefixStore,
    cw_std::{BlockInfo, Context, Event, Storage, Tx},
    cw_vm::Instance,
    tracing::{debug, warn},
};

pub fn authenticate_tx<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> AppResult<Vec<Event>> {
    match _authenticate_tx(store, block, tx) {
        Ok(events) => {
            // TODO: add txhash here?
            debug!(sender = tx.sender.to_string(), "Transaction authenticated");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to authenticate transaction");
            Err(err)
        },
    }
}

fn _authenticate_tx<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    tx:    &Tx,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &tx.sender)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &tx.sender]);
    let querier = Querier::new(store, block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call `before_tx` entry point
    let ctx = Context {
        chain_id,
        block:    block.clone(),
        contract: tx.sender.clone(),
        sender:   None,
        funds:    None,
        simulate: Some(false),
    };
    let resp = instance.call_before_tx(&ctx, tx)?.into_std_result()?;

    debug_assert!(resp.messages.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(vec![new_before_tx_event(&tx.sender, resp)])
}
