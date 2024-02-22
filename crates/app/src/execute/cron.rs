use {
    super::{handle_submessages, new_after_block_event, new_before_block_event},
    crate::{AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{Addr, BlockInfo, Context, Event, Storage},
    cw_vm::Instance,
    tracing::{debug, warn},
};

// ------------------------------- before block --------------------------------

pub fn before_block<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>> {
    match _before_block(store, block, contract) {
        Ok(events) => {
            debug!(contract = contract.to_string(), "Called before block hook");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call before block hook");
            Err(err)
        },
    }
}

fn _before_block<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, contract]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call the recipient contract's `before_block` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        contract.clone(),
        sender:          None,
        funds:           None,
        simulate:        None,
        submsg_result:   None,
    };
    let resp = instance.call_before_block(&ctx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_before_block_event(contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}

// -------------------------------- after block --------------------------------

pub fn after_block<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>> {
    match _after_block(store, block, contract) {
        Ok(events) => {
            debug!(contract = contract.to_string(), "Called after block hook");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call after block hook");
            Err(err)
        },
    }
}

fn _after_block<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, contract]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call the recipient contract's `after_block` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        contract.clone(),
        sender:          None,
        funds:           None,
        simulate:        None,
        submsg_result:   None,
    };
    let resp = instance.call_after_block(&ctx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_after_block_event(contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
