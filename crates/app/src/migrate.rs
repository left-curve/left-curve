use {
    crate::{
        create_vm_instance, handle_submessages, load_program, new_migrate_event, AppError,
        AppResult, Vm, ACCOUNTS, CHAIN_ID,
    },
    grug_types::{Addr, BlockInfo, Context, Event, Hash, Json, Storage},
    tracing::{info, warn},
};

pub fn do_migrate<VM>(
    store:         Box<dyn Storage>,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           &Json,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_migrate::<VM>(store, block, contract, sender, new_code_hash, msg) {
        Ok(events) => {
            info!(contract = contract.to_string(), "Migrated contract");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to execute contract");
            Err(err)
        },
    }
}

fn _do_migrate<VM>(
    mut store:     Box<dyn Storage>,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           &Json,
) -> AppResult<Vec<Event>>
where
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&store)?;
    let mut account = ACCOUNTS.load(&store, contract)?;

    // only the admin can update code hash
    let Some(admin) = &account.admin else {
        return Err(AppError::AdminNotSet);
    };
    if sender != admin {
        return Err(AppError::not_admin(sender.clone(), admin.clone()));
    }

    // save the new code hash
    let old_code_hash = account.code_hash;
    account.code_hash = new_code_hash;
    ACCOUNTS.save(&mut store, contract, &account)?;

    // create VM instance
    let program = load_program::<VM>(&store, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(store.clone(), block.clone(), contract, program)?;

    // call the contract's migrate entry point
    let ctx = Context {
        chain_id,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        contract.clone(),
        sender:          Some(sender.clone()),
        funds:           None,
        simulate:        None,
    };
    let resp = instance.call_migrate(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_migrate_event(
        &ctx.contract,
        &old_code_hash,
        &account.code_hash,
        resp.attributes,
    )];
    events.extend(handle_submessages::<VM>(store, block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
