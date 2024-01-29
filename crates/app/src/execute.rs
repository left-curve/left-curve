use {
    crate::{
        handle_submessages, new_execute_event, new_instantiate_event, new_migrate_event,
        new_receive_event, new_store_code_event, new_transfer_event, new_update_config_event,
        AppError, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE,
    },
    cw_db::PrefixStore,
    cw_std::{
        hash, Account, Addr, Binary, BlockInfo, Coins, Config, Context, Event, Hash, Message,
        Storage, TransferMsg,
    },
    cw_vm::Instance,
    tracing::{info, warn},
};

pub fn process_msg<S: Storage + Clone + 'static>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    msg:       Message,
) -> AppResult<Vec<Event>> {
    match msg {
        Message::UpdateConfig {
            new_cfg,
        } => update_config(&mut store, sender, &new_cfg),
        Message::Transfer {
            to,
            coins,
        } => transfer(store, block, sender.clone(), to, coins),
        Message::StoreCode {
            wasm_byte_code,
        } => store_code(&mut store, sender, &wasm_byte_code),
        Message::Instantiate {
            code_hash,
            msg,
            salt,
            funds,
            admin,
        } => instantiate(store, block, sender, code_hash, msg, salt, funds, admin),
        Message::Execute {
            contract,
            msg,
            funds,
        } => execute(store, block, &contract, sender, msg, funds),
        Message::Migrate {
            contract,
            new_code_hash,
            msg,
        } => migrate(store, block, &contract, sender, new_code_hash, msg),
    }
}

// ------------------------------- update config -------------------------------

fn update_config(
    store:   &mut dyn Storage,
    sender:  &Addr,
    new_cfg: &Config,
) -> AppResult<Vec<Event>> {
    match _update_config(store, sender, new_cfg) {
        Ok(events) => {
            info!("Config updated");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to update config");
            Err(err)
        },
    }
}

fn _update_config(
    store:   &mut dyn Storage,
    sender:  &Addr,
    new_cfg: &Config,
) -> AppResult<Vec<Event>> {
    // make sure the sender is authorized to update the config
    let cfg = CONFIG.load(store)?;
    let Some(owner) = cfg.owner else {
        return Err(AppError::OwnerNotSet);
    };
    if sender != &owner {
        return Err(AppError::not_owner(sender.clone(), owner));
    }

    // save the new config
    CONFIG.save(store, new_cfg)?;

    Ok(vec![new_update_config_event(sender)])
}

// -------------------------------- store code ---------------------------------

fn store_code(
    store:          &mut dyn Storage,
    uploader:       &Addr,
    wasm_byte_code: &Binary,
) -> AppResult<Vec<Event>> {
    match _store_code(store, uploader, wasm_byte_code) {
        Ok((events, code_hash)) => {
            info!(code_hash = code_hash.to_string(), "Stored code");
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to store code");
            Err(err)
        },
    }
}

// return the hash of the code that is stored, for purpose of tracing/logging
fn _store_code(
    store:          &mut dyn Storage,
    uploader:       &Addr,
    wasm_byte_code: &Binary,
) -> AppResult<(Vec<Event>, Hash)> {
    // TODO: static check, ensure wasm code has necessary imports/exports
    let code_hash = hash(wasm_byte_code);

    // make sure that the same code isn't uploaded twice
    if CODES.has(store, &code_hash) {
        return Err(AppError::code_exists(code_hash));
    }

    CODES.save(store, &code_hash, wasm_byte_code)?;

    Ok((vec![new_store_code_event(&code_hash, uploader)], code_hash))
}

// --------------------------------- transfer ----------------------------------

fn transfer<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    from:  Addr,
    to:    Addr,
    coins: Coins,
) -> AppResult<Vec<Event>> {
    match _transfer(store, block, from, to, coins) {
        Ok((events, msg)) => {
            info!(
                from  = msg.from.to_string(),
                to    = msg.to.to_string(),
                coins = msg.coins.to_string(),
                "Transferred coins"
            );
            Ok(events)
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to transfer coins");
            Err(err)
        },
    }
}

// return the TransferMsg, which includes the sender, receiver, and amount, for
// purpose of tracing/logging
fn _transfer<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    from:  Addr,
    to:    Addr,
    coins: Coins,
) -> AppResult<(Vec<Event>, TransferMsg)> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let cfg = CONFIG.load(&store)?;
    let account = ACCOUNTS.load(&store, &cfg.bank)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &cfg.bank]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call transfer
    let ctx = Context {
        chain_id,
        block:    block.clone(),
        contract: cfg.bank,
        sender:   None,
        funds:    None,
        simulate: None,
    };
    let msg = TransferMsg {
        from,
        to,
        coins,
    };
    let resp = instance.call_transfer(&ctx, &msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_transfer_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(store.clone(), block, &ctx.contract, resp.messages)?);

    // call the recipient contract's `receive` entry point to inform it of this
    // transfer
    _receive(store, block, msg, events)
}

fn _receive<S: Storage + Clone + 'static>(
    store:      S,
    block:      &BlockInfo,
    msg:        TransferMsg,
    mut events: Vec<Event>,
) -> AppResult<(Vec<Event>, TransferMsg)> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &msg.to)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &msg.to]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call the recipient contract's `receive` entry point
    let ctx = Context {
        chain_id,
        block:    block.clone(),
        contract: msg.to.clone(),
        sender:   Some(msg.from.clone()),
        funds:    Some(msg.coins.clone()),
        simulate: None,
    };
    let resp = instance.call_receive(&ctx)?.into_std_result()?;

    // handle submessages
    events.push(new_receive_event(&msg.to, resp.attributes));
    events.extend(handle_submessages(store, block, &ctx.contract, resp.messages)?);

    Ok((events, msg))
}

// -------------------------------- instantiate --------------------------------

#[allow(clippy::too_many_arguments)]
fn instantiate<S: Storage + Clone + 'static>(
    store:     S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> AppResult<Vec<Event>> {
    match _instantiate(store, block, sender, code_hash, msg, salt, funds, admin) {
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
fn _instantiate<S: Storage + Clone + 'static>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    code_hash: Hash,
    msg:       Binary,
    salt:      Binary,
    funds:     Coins,
    admin:     Option<Addr>,
) -> AppResult<(Vec<Event>, Addr)> {
    let chain_id = CHAIN_ID.load(&store)?;

    // load wasm code
    let wasm_byte_code = CODES.load(&store, &code_hash)?;

    // compute contract address and save account info
    let address = Addr::compute(sender, &code_hash, &salt);

    // there can't already be an account of the same address
    if ACCOUNTS.has(&store, &address) {
        return Err(AppError::account_exists(address));
    }

    let account = Account { code_hash, admin };
    ACCOUNTS.save(&mut store, &address, &account)?;

    // make the coin transfers
    if !funds.is_empty() {
        _transfer(store.clone(), block, sender.clone(), address.clone(), funds.clone())?;
    }

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &address]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call instantiate
    let ctx = Context {
        chain_id,
        block:    block.clone(),
        contract: address,
        sender:   Some(sender.clone()),
        funds:    Some(funds),
        simulate: None,
    };
    let resp = instance.call_instantiate(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_instantiate_event(&ctx.contract, &account.code_hash, resp.attributes)];
    events.extend(handle_submessages(store, block, &ctx.contract, resp.messages)?);

    Ok((events, ctx.contract))
}

// ---------------------------------- execute ----------------------------------

fn execute<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      Binary,
    funds:    Coins,
) -> AppResult<Vec<Event>> {
    match _execute(store, block, contract, sender, msg, funds) {
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

fn _execute<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      Binary,
    funds:    Coins,
) -> AppResult<Vec<Event>> {
    let chain_id = CHAIN_ID.load(&store)?;

    // make the coin transfers
    if !funds.is_empty() {
        _transfer(store.clone(), block, sender.clone(), contract.clone(), funds.clone())?;
    }

    // load wasm code
    let account = ACCOUNTS.load(&store, contract)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call execute
    let ctx = Context {
        chain_id,
        block:    block.clone(),
        contract: contract.clone(),
        sender:   Some(sender.clone()),
        funds:    Some(funds),
        simulate: None,
    };
    let resp = instance.call_execute(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_execute_event(&ctx.contract, resp.attributes)];
    events.extend(handle_submessages(store, block, &ctx.contract, resp.messages)?);

    Ok(events)
}

// ---------------------------------- migrate ----------------------------------

fn migrate<S: Storage + Clone + 'static>(
    store:         S,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           Binary,
) -> AppResult<Vec<Event>> {
    match _migrate(store, block, contract, sender, new_code_hash, msg) {
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

fn _migrate<S: Storage + Clone + 'static>(
    mut store:     S,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           Binary,
) -> AppResult<Vec<Event>> {
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

    // load wasm code
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store.clone(), block.clone());
    let mut instance = Instance::build_from_code(substore, querier, &wasm_byte_code)?;

    // call the contract's migrate entry point
    let ctx = Context {
        chain_id,
        block:    block.clone(),
        contract: contract.clone(),
        sender:   Some(sender.clone()),
        funds:    None,
        simulate: None,
    };
    let resp = instance.call_migrate(&ctx, msg)?.into_std_result()?;

    // handle submessages
    let mut events = vec![new_migrate_event(
        &ctx.contract,
        &old_code_hash,
        &account.code_hash,
        resp.attributes,
    )];
    events.extend(handle_submessages(store, block, &ctx.contract, resp.messages)?);

    Ok(events)
}
