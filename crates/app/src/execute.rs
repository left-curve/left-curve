use {
    crate::{
        create_vm_instance, do_transfer, handle_submessages, load_program, new_execute_event,
        AppError, AppResult, Vm, ACCOUNTS, CHAIN_ID,
    },
    grug_types::{Addr, BlockInfo, Coins, Context, Event, Json, Storage},
    tracing::{info, warn},
};

pub fn do_execute<VM>(
    storage:    Box<dyn Storage>,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      &Json,
    funds:    Coins,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_execute::<VM>(storage, block, contract, sender, msg, funds) {
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

fn _do_execute<VM>(
    storage:    Box<dyn Storage>,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      &Json,
    funds:    Coins,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, contract)?;

    // make the coin transfers
    if !funds.is_empty() {
        do_transfer::<VM>(
            storage.clone(),
            block,
            sender.clone(),
            contract.clone(),
            funds.clone(),
            false,
        )?;
    }

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), contract, program)?;

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
    events.extend(handle_submessages::<VM>(storage, block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
