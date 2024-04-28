use {
    crate::{
        create_vm_instance, handle_submessages, load_program, new_migrate_event, AppError,
        AppResult, ACCOUNTS, CHAIN_ID,
    },
    cw_std::{Addr, BlockInfo, Context, Event, Hash, Json, Storage, Vm},
    tracing::{info, warn},
};

pub fn do_migrate<S, VM>(
    store:         S,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           &Json,
) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_migrate::<S, VM>(store, block, contract, sender, new_code_hash, msg) {
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

fn _do_migrate<S, VM>(
    mut store:     S,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           &Json,
) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
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
    let mut instance = create_vm_instance::<S, VM>(store.clone(), block.clone(), contract, program)?;

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
    events.extend(handle_submessages::<VM>(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
