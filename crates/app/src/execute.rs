use {
    crate::{
        create_vm_instance, do_transfer, handle_submessages, load_program, new_execute_event,
        AppError, AppResult, Vm, ACCOUNTS, CHAIN_ID,
    },
    cw_types::{Addr, BlockInfo, Coins, Context, Event, Json, Storage},
    tracing::{info, warn},
};

pub fn do_execute<VM>(
    store:    Box<dyn Storage>,
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
    match _do_execute::<VM>(store, block, contract, sender, msg, funds) {
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
    store:    Box<dyn Storage>,
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
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, contract)?;

    // make the coin transfers
    if !funds.is_empty() {
        do_transfer::<VM>(
            store.clone(),
            block,
            sender.clone(),
            contract.clone(),
            funds.clone(),
            false,
        )?;
    }

    let program = load_program::<VM>(&store, &account.code_hash)?;
    let mut instance = create_vm_instance::<VM>(store.clone(), block.clone(), contract, program)?;

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
    events.extend(handle_submessages::<VM>(store, block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
