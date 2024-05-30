use {
    crate::{
        create_vm_instance, handle_submessages, has_permission, load_program,
        new_client_misbehavior_event, new_create_client_event, new_update_client_event, AppError,
        AppResult, Vm, ACCOUNTS, CHAIN_ID, CONFIG,
    },
    grug_types::{
        Account, Addr, Binary, BlockInfo, Context, Event, Hash, IbcClientUpdateMsg, Json, Storage,
    },
    tracing::{info, warn},
};

// ------------------------------- create client -------------------------------

pub fn do_client_create<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    sender: &Addr,
    code_hash: Hash,
    client_state: Json,
    consensus_state: Json,
    salt: Binary,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    match _do_client_create::<VM>(
        storage,
        block,
        sender,
        code_hash,
        client_state,
        consensus_state,
        salt,
    ) {
        Ok((events, address)) => {
            info!(address = address.to_string(), "Create IBC client");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to create IBC client");
            Err(err)
        },
    }
}

pub fn _do_client_create<VM>(
    mut storage: Box<dyn Storage>,
    block: &BlockInfo,
    sender: &Addr,
    code_hash: Hash,
    client_state: Json,
    consensus_state: Json,
    salt: Binary,
) -> AppResult<(Vec<Event>, Addr)>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;

    // make sure:
    // 1. the sender has the permission to create clients
    // 2. the code hash is allowed as IBC client
    let cfg = CONFIG.load(&storage)?;
    if !has_permission(&cfg.permissions.create_client, cfg.owner.as_ref(), sender) {
        return Err(AppError::Unauthorized);
    }
    if !cfg.allowed_clients.contains(&code_hash) {
        return Err(AppError::not_allowed_client(code_hash));
    };

    // compute contract address and make sure there can't already be an account
    // of the same address
    let address = Addr::compute(sender, &code_hash, &salt);
    if ACCOUNTS.has(&storage, &address) {
        return Err(AppError::account_exists(address));
    }

    // save the account info now that we know there's no duplicate
    let account = Account {
        code_hash,
        // IBC clients are not upgradable
        admin: None,
    };
    ACCOUNTS.save(&mut storage, &address, &account)?;

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), &address, program)?;

    // call `ibc_client_create` entry point
    let ctx = Context {
        chain_id,
        block_height: block.height,
        block_timestamp: block.timestamp,
        block_hash: block.hash.clone(),
        contract: address,
        sender: Some(sender.clone()),
        funds: None,
        simulate: None,
    };
    let resp = instance
        .call_ibc_client_create(&ctx, &client_state, &consensus_state)?
        .into_std_result()?;

    // handle submessages
    let mut events = vec![new_create_client_event(
        &ctx.contract,
        &account.code_hash,
        resp.attributes,
    )];
    events.extend(handle_submessages::<VM>(
        storage,
        block,
        sender,
        resp.submsgs,
    )?);

    Ok((events, ctx.contract))
}

// ------------------------------- update client -------------------------------

pub fn do_client_update<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    sender: &Addr,
    client_id: &Addr,
    header: Json,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    match _do_client_update::<VM>(storage, block, sender, client_id, header) {
        Ok(events) => {
            info!(client_id = client_id.to_string(), "Update IBC client");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to update IBC client");
            Err(err)
        },
    }
}

fn _do_client_update<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    sender: &Addr,
    client_id: &Addr,
    header: Json,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, client_id)?;

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), client_id, program)?;

    // call `ibc_client_update` entry point
    let ctx = Context {
        chain_id,
        block_height: block.height,
        block_timestamp: block.timestamp,
        block_hash: block.hash.clone(),
        contract: client_id.clone(),
        sender: Some(sender.clone()),
        funds: None,
        simulate: None,
    };
    let msg = IbcClientUpdateMsg::Update { header };
    let resp = instance
        .call_ibc_client_update(&ctx, &msg)?
        .into_std_result()?;

    // handle submessages
    let mut events = vec![new_update_client_event(
        &ctx.contract,
        &account.code_hash,
        resp.attributes,
    )];
    events.extend(handle_submessages::<VM>(
        storage,
        block,
        &ctx.contract,
        resp.submsgs,
    )?);

    Ok(events)
}

// ------------------------------- freeze client -------------------------------

pub fn do_client_freeze<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    sender: &Addr,
    client_id: &Addr,
    misbehavior: Json,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    match _do_client_freeze::<VM>(storage, block, sender, client_id, misbehavior) {
        Ok(events) => {
            warn!(
                client = client_id.to_string(),
                "Froze IBC client due to misbehavior"
            );
            Ok(events)
        },
        Err(err) => {
            warn!(
                err = err.to_string(),
                "Failed to freeze IBC client due to misbehavior"
            );
            Err(err)
        },
    }
}

fn _do_client_freeze<VM>(
    storage: Box<dyn Storage>,
    block: &BlockInfo,
    sender: &Addr,
    client_id: &Addr,
    misbehavior: Json,
) -> AppResult<Vec<Event>>
where
    VM: Vm,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let account = ACCOUNTS.load(&storage, client_id)?;

    let program = load_program::<VM>(&storage, &account.code_hash)?;
    let instance = create_vm_instance::<VM>(storage.clone(), block.clone(), client_id, program)?;

    // call `ibc_client_update` entry point
    let ctx = Context {
        chain_id,
        block_height: block.height,
        block_timestamp: block.timestamp,
        block_hash: block.hash.clone(),
        contract: client_id.clone(),
        sender: Some(sender.clone()),
        funds: None,
        simulate: None,
    };
    let msg = IbcClientUpdateMsg::UpdateOnMisbehavior { misbehavior };
    let resp = instance
        .call_ibc_client_update(&ctx, &msg)?
        .into_std_result()?;

    // handle submessages
    let mut events = vec![new_client_misbehavior_event(
        &ctx.contract,
        &account.code_hash,
        resp.attributes,
    )];
    events.extend(handle_submessages::<VM>(
        storage,
        block,
        &ctx.contract,
        resp.submsgs,
    )?);

    Ok(events)
}
