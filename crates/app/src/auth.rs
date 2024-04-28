use {
    crate::{
        create_vm_instance, handle_submessages, load_program, new_after_tx_event,
        new_before_tx_event, AppError, AppResult, ACCOUNTS, CHAIN_ID,
    },
    cw_std::{BlockInfo, Context, Event, Storage, Tx, Vm},
    tracing::{debug, warn},
};

// --------------------------------- before tx ---------------------------------

pub fn do_before_tx<S, VM>(store: S, block: &BlockInfo, tx: &Tx) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_before_tx::<S, VM>(store, block, tx) {
        Ok(events) => {
            // TODO: add txhash here?
            debug!(sender = tx.sender.to_string(), "Called before transaction hook");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call before transaction hook");
            Err(err)
        },
    }
}

fn _do_before_tx<S, VM>(store: S, block: &BlockInfo, tx: &Tx) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &tx.sender)?;

    let program = load_program::<VM>(&store, &account.code_hash)?;
    let mut instance = create_vm_instance::<S, VM>(store.clone(), block.clone(), &tx.sender, program)?;

    // call `before_tx` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        tx.sender.clone(),
        sender:          None,
        funds:           None,
        simulate:        Some(false),
    };
    let resp = instance.call_before_tx(&ctx, tx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_before_tx_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages::<VM>(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}

// --------------------------------- after tx ----------------------------------

pub fn do_after_tx<S, VM>(store: S, block: &BlockInfo, tx: &Tx) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_after_tx::<S, VM>(store, block, tx) {
        Ok(events) => {
            // TODO: add txhash here?
            debug!(sender = tx.sender.to_string(), "Called after transaction hook");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to call after transaction hook");
            Err(err)
        },
    }
}

fn _do_after_tx<S, VM>(store: S, block: &BlockInfo, tx: &Tx) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &tx.sender)?;

    let program = load_program::<VM>(&store, &account.code_hash)?;
    let mut instance = create_vm_instance::<S, VM>(store.clone(), block.clone(), &tx.sender, program)?;

    // call `after_tx` entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        tx.sender.clone(),
        sender:          None,
        funds:           None,
        simulate:        Some(false),
    };
    let resp = instance.call_after_tx(&ctx, tx)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_after_tx_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages::<VM>(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
