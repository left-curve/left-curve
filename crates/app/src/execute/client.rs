use {
    super::{handle_submessages, has_permission, new_create_client_event},
    crate::{AppError, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{Account, Addr, Binary, BlockInfo, Context, Event, Hash, Storage},
    cw_vm::Instance,
    tracing::{info, warn},
};

// ------------------------------- create client -------------------------------

pub fn create_client<S: Storage + Clone + 'static>(
    store:           S,
    block:           &BlockInfo,
    sender:          &Addr,
    code_hash:       Hash,
    client_state:    Binary,
    consensus_state: Binary,
    salt:            Binary,
) -> AppResult<Vec<Event>> {
    match _create_client(store, block, sender, code_hash, client_state, consensus_state, salt) {
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

pub fn _create_client<S: Storage + Clone + 'static>(
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

    // call wasm export `ibc_client_create`
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

// ---------------------------- submit misbehavior -----------------------------
