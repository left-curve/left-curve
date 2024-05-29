use {
    crate::{
        create_vm_instance, handle_submessages, load_program, new_after_block_event,
        new_before_block_event, AppError, AppResult, Vm, ACCOUNTS, CHAIN_ID,
    },
    grug_types::{Addr, BlockInfo, Context, Event, Storage},
    tracing::{info, warn},
};

// ------------------------------- before block --------------------------------

pub fn do_before_block<VM>(
    storage:    Box<dyn Storage>,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_before_block::<VM>(storage, block, contract) {
        Ok(events) => {
            info!(contract = contract.to_string(), "Called before block hook");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call before block hook");
            Err(err)
        },
    }
}

fn _do_before_block<VM>(
    storage:    Box<dyn Storage>,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, contract)?;

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), contract, program)?;

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
    };
    let resp = instance.call_before_block(&ctx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_before_block_event(contract, resp.attributes)];
    events.extend(handle_submessages::<VM>(storage, block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}

// -------------------------------- after block --------------------------------

pub fn do_after_block<VM>(
    storage:    Box<dyn Storage>,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_after_block::<VM>(storage, block, contract) {
        Ok(events) => {
            info!(contract = contract.to_string(), "Called after block hook");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call after block hook");
            Err(err)
        },
    }
}

fn _do_after_block<VM>(
    storage:    Box<dyn Storage>,
    block:    &BlockInfo,
    contract: &Addr,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, contract)?;

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), contract, program)?;

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
    };
    let resp = instance.call_after_block(&ctx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_after_block_event(contract, resp.attributes)];
    events.extend(handle_submessages::<VM>(storage, block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
