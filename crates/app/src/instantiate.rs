use {
    crate::{
        create_vm_instance, do_transfer, handle_submessages, has_permission, load_program,
        new_instantiate_event, AppError, AppResult, ACCOUNTS, CHAIN_ID, CONFIG,
    },
    cw_types::{Account, Addr, Binary, BlockInfo, Coins, Context, Event, Hash, Json, Storage, Vm},
    tracing::{info, warn},
};

#[allow(clippy::too_many_arguments)]
pub fn do_instantiate<S, VM>(
    store:     S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       &Json,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> AppResult<Vec<Event>>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    match _do_instantiate::<S, VM>(store, block, sender, code_hash, msg, salt, funds, admin) {
        Ok((events, address)) => {
            info!(address = address.to_string(), "Instantiated contract");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to instantiate contract");
            Err(err)
        },
    }
}

// return the address of the contract that is instantiated.
#[allow(clippy::too_many_arguments)]
fn _do_instantiate<S, VM>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       &Json,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> AppResult<(Vec<Event>, Addr)>
where
    S: Storage + Clone + 'static,
    VM: Vm + 'static,
    AppError: From<VM::Error>,
{
    // make sure the user has permission to instantiate contracts
    let cfg = CONFIG.load(&store)?;
    if !has_permission(&cfg.permissions.instantiate, cfg.owner.as_ref(), sender) {
        return Err(AppError::Unauthorized);
    }

    // compute contract address and make sure there can't already be an account
    // of the same address
    let address = Addr::compute(sender, &code_hash, &salt);
    if ACCOUNTS.has(&store, &address) {
        return Err(AppError::account_exists(address));
    }

    // save the account info now that we know there's no duplicate
    let account = Account { code_hash, admin };
    ACCOUNTS.save(&mut store, &address, &account)?;

    // make the coin transfers
    if !funds.is_empty() {
        do_transfer::<_, VM>(
            store.clone(),
            block,
            sender.clone(),
            address.clone(),
            funds.clone(),
            false,
        )?;
    }

    // create VM instance
    let program = load_program::<VM>(&store, &account.code_hash)?;
    let mut instance = create_vm_instance::<S, VM>(store.clone(), block.clone(), &address, program)?;

    // call instantiate
    let ctx = Context {
        chain_id:        CHAIN_ID.load(&store)?,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        address,
        sender:          Some(sender.clone()),
        funds:           Some(funds),
        simulate:        None,
    };
    let resp = instance.call_instantiate(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_instantiate_event(&ctx.contract, &account.code_hash, resp.attributes)];
    events.extend(handle_submessages::<VM>(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok((events, ctx.contract))
}
