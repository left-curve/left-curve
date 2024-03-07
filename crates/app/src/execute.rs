use {
    super::{do_transfer, handle_submessages, new_execute_event},
    crate::{AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{Addr, Binary, BlockInfo, Coins, Context, Event, Storage},
    cw_vm::Instance,
    tracing::{info, warn},
};

pub fn do_execute<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      Binary,
    funds:    Coins,
) -> AppResult<Vec<Event>> {
    match _do_execute(store, block, contract, sender, msg, funds) {
        Ok(events) => {
            info!(contract = contract.to_string(), "Executed contract");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to execute contract");
            Err(err)
        },
    }
}

fn _do_execute<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      Binary,
    funds:    Coins,
) -> AppResult<Vec<Event>> {
    let chain_id = CHAIN_ID.load(&store)?;

    // make the coin transfers
    if !funds.is_empty() {
        do_transfer(
            store.clone(),
            block,
            sender.clone(),
            contract.clone(),
            funds.clone(),
            false,
        )?;
    }

    // load wasm code
    let account = ACCOUNTS.load(&store, contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call execute
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        contract.clone(),
        sender:          Some(sender.clone()),
        funds:           Some(funds),
        simulate:        None,
    };
    let resp = instance.call_execute(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_execute_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
