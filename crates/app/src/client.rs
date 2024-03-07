use {
    super::{
        handle_submessages, has_permission, new_client_misbehavior_event, new_create_client_event,
        new_update_client_event,
    },
    crate::{AppError, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{Account, Addr, Binary, BlockInfo, Context, Event, Hash, IbcClientUpdateMsg, Storage},
    cw_vm::Instance,
    tracing::{info, warn},
};

// ------------------------------- create client -------------------------------

pub fn do_create_client<S: Storage + Clone + 'static>(
    store:           S,
    block:           &BlockInfo,
    sender:          &Addr,
    code_hash:       Hash,
    client_state:    Binary,
    consensus_state: Binary,
    salt:            Binary,
) -> AppResult<Vec<Event>> {
    match _do_create_client(store, block, sender, code_hash, client_state, consensus_state, salt) {
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

pub fn _do_create_client<S: Storage + Clone + 'static>(
    mut store:       S,
    block:           &BlockInfo,
    sender:          &Addr,
    code_hash:       Hash,
    client_state:    Binary,
    consensus_state: Binary,
    salt:            Binary,
) -> AppResult<(Vec<Event>, Addr)> {
    // make sure:
    // 1. the sender has the permission to create clients
    // 2. the code hash is allowed as IBC client
    let cfg = CONFIG.load(&store)?;
    if !has_permission(&cfg.permissions.create_client, cfg.owner.as_ref(), sender) {
        return Err(AppError::Unauthorized);
    }
    if !cfg.allowed_clients.contains(&code_hash) {
        return Err(AppError::not_allowed_client(code_hash));
    };

    // compute contract address and make sure there can't already be an account
    // of the same address
    let address = Addr::compute(sender, &code_hash, &salt);
    if ACCOUNTS.has(&store, &address) {
        return Err(AppError::account_exists(address));
    }

    // load wasm code
    let wasm_byte_code = CODES.load(&store, &code_hash)?;

    // save the account info now that we know there's no duplicate
    let account = Account {
        code_hash,
        // IBC clients are not upgradable
        admin: None,
    };
    ACCOUNTS.save(&mut store, &address, &account)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &address]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call `ibc_client_create` entry point
    let ctx = Context {
        chain_id:        CHAIN_ID.load(&store)?,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        address,
        sender:          Some(sender.clone()),
        funds:           None,
        simulate:        None,
    };
    let resp = instance.call_ibc_client_create(&ctx, &client_state, &consensus_state)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_create_client_event(&ctx.contract, &account.code_hash, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, sender, resp.submsgs)?);

    Ok((events, ctx.contract))
}

// ------------------------------- update client -------------------------------

pub fn do_update_client<S: Storage + Clone + 'static>(
    store:  S,
    block:  &BlockInfo,
    sender: &Addr,
    client: &Addr,
    header: Binary,
) -> AppResult<Vec<Event>> {
    match _do_update_client(store, block, sender, client, header) {
        Ok(events) => {
            info!(client = client.to_string(), "Update IBC client");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to update IBC client");
            Err(err)
        },
    }
}

fn _do_update_client<S: Storage + Clone + 'static>(
    store:  S,
    block:  &BlockInfo,
    sender: &Addr,
    client: &Addr,
    header: Binary,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let account = ACCOUNTS.load(&store, client)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &client]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call `ibc_client_update` entry point
    let ctx = Context {
        chain_id:        CHAIN_ID.load(&store)?,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        client.clone(),
        sender:          Some(sender.clone()),
        funds:           None,
        simulate:        None,
    };
    let msg = IbcClientUpdateMsg::Update {
        header,
    };
    let resp = instance.call_ibc_client_update(&ctx, &msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_update_client_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}

// ---------------------------- submit misbehavior -----------------------------

pub fn do_submit_misbehavior<S: Storage + Clone + 'static>(
    store:       S,
    block:       &BlockInfo,
    sender:      &Addr,
    client:      &Addr,
    misbehavior: Binary,
) -> AppResult<Vec<Event>> {
    match _do_submit_misbehavior(store, block, sender, client, misbehavior) {
        Ok(events) => {
            warn!(client = client.to_string(), "Froze IBC client due to misbehavior");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to freeze IBC client due to misbehavior");
            Err(err)
        },
    }
}

fn _do_submit_misbehavior<S: Storage + Clone + 'static>(
    store:       S,
    block:       &BlockInfo,
    sender:      &Addr,
    client:      &Addr,
    misbehavior: Binary,
) -> AppResult<Vec<Event>> {
    // load wasm code
    let account = ACCOUNTS.load(&store, client)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &client]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call `ibc_client_update` entry point
    let ctx = Context {
        chain_id:        CHAIN_ID.load(&store)?,
        block_height:    block.height,
        block_timestamp: block.timestamp,
        block_hash:      block.hash.clone(),
        contract:        client.clone(),
        sender:          Some(sender.clone()),
        funds:           None,
        simulate:        None,
    };
    let msg = IbcClientUpdateMsg::UpdateOnMisbehavior {
        misbehavior,
    };
    let resp = instance.call_ibc_client_update(&ctx, &msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_client_misbehavior_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(Box::new(store), block, &ctx.contract, resp.submsgs)?);

    Ok(events)
}
