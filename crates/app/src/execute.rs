use {
    crate::{AppError, AppResult, Querier, ACCOUNTS, CHAIN_ID, CODES, CONFIG, CONTRACT_NAMESPACE},
    cw_db::PrefixStore,
    cw_std::{
        hash, Account, Addr, Binary, BlockInfo, Coins, Config, Context, Hash, Message, Storage,
        TransferMsg,
    },
    cw_vm::Instance,
    tracing::{info, warn},
};

pub fn process_msg<S: Storage + Clone + 'static>(
    mut store: S,
    block:     &BlockInfo,
    sender:    &Addr,
    msg:       Message,
) -> AppResult<()> {
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
        } => store_code(&mut store, &wasm_byte_code),
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

fn update_config(store: &mut dyn Storage, sender: &Addr, new_cfg: &Config) -> AppResult<()> {
    match _update_config(store, sender, new_cfg) {
        Ok(()) => {
            info!("Config updated");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to update config");
            Err(err)
        },
    }
}

fn _update_config(store: &mut dyn Storage, sender: &Addr, new_cfg: &Config) -> AppResult<()> {
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

    Ok(())
}

// -------------------------------- store code ---------------------------------

fn store_code(store: &mut dyn Storage, wasm_byte_code: &Binary) -> AppResult<()> {
    match _store_code(store, wasm_byte_code) {
        Ok(code_hash) => {
            info!(code_hash = code_hash.to_string(), "Stored code");
            Ok(())
        },
        Err(err) => {
            warn!(err = err.to_string(), "Failed to store code");
            Err(err)
        },
    }
}

// return the hash of the code that is stored, for purpose of tracing/logging
fn _store_code(store: &mut dyn Storage, wasm_byte_code: &Binary) -> AppResult<Hash> {
    // TODO: static check, ensure wasm code has necessary imports/exports
    let code_hash = hash(wasm_byte_code);

    // make sure that the same code isn't uploaded twice
    if CODES.has(store, &code_hash) {
        return Err(AppError::code_exists(code_hash));
    }

    CODES.save(store, &code_hash, wasm_byte_code)?;

    Ok(code_hash)
}

// --------------------------------- transfer ----------------------------------

fn transfer<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    from:  Addr,
    to:    Addr,
    coins: Coins,
) -> AppResult<()> {
    match _transfer(store, block, from, to, coins) {
        Ok(TransferMsg { from, to, coins }) => {
            info!(
                from  = from.to_string(),
                to    = to.to_string(),
                coins = coins.to_string(),
                "Transferred coins"
            );
            Ok(())
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
) -> AppResult<TransferMsg> {
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

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    // call the recipient contract's `receive` entry point to inform it of this
    // transfer
    _receive(store, block, msg)
}

fn _receive<S: Storage + Clone + 'static>(
    store: S,
    block: &BlockInfo,
    msg:   TransferMsg,
) -> AppResult<TransferMsg> {
    // load wasm code
    let chain_id = CHAIN_ID.load(&store)?;
    let account = ACCOUNTS.load(&store, &msg.to)?;
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &msg.to]);
    let querier = Querier::new(store, block.clone());
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

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(msg)
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
) -> AppResult<()> {
    match _instantiate(store, block, sender, code_hash, msg, salt, funds, admin) {
        Ok(address) => {
            info!(address = address.to_string(), "Instantiated contract");
            Ok(())
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
) -> AppResult<Addr> {
    let chain_id = CHAIN_ID.load(&store)?;

    // load wasm code
    let wasm_byte_code = CODES.load(&store, &code_hash)?;

    // compute contract address and save account info
    let address = Addr::compute(sender, &code_hash, &salt);

    // there can't already be an account of the same address
    if ACCOUNTS.has(&store, &address) {
        return Err(AppError::account_exists(address));
    }

    ACCOUNTS.save(&mut store, &address, &Account { code_hash, admin })?;

    // make the coin transfers
    if !funds.is_empty() {
        _transfer(store.clone(), block, sender.clone(), address.clone(), funds.clone())?;
    }

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &address]);
    let querier = Querier::new(store, block.clone());
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

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(ctx.contract)
}

// ---------------------------------- execute ----------------------------------

fn execute<S: Storage + Clone + 'static>(
    store:    S,
    block:    &BlockInfo,
    contract: &Addr,
    sender:   &Addr,
    msg:      Binary,
    funds:    Coins,
) -> AppResult<()> {
    match _execute(store, block, contract, sender, msg, funds) {
        Ok(()) => {
            info!(contract = contract.to_string(), "Executed contract");
            Ok(())
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
) -> AppResult<()> {
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
    let querier = Querier::new(store, block.clone());
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

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(())
}

// ---------------------------------- migrate ----------------------------------

fn migrate<S: Storage + Clone + 'static>(
    store:         S,
    block:         &BlockInfo,
    contract:      &Addr,
    sender:        &Addr,
    new_code_hash: Hash,
    msg:           Binary,
) -> AppResult<()> {
    match _migrate(store, block, contract, sender, new_code_hash, msg) {
        Ok(()) => {
            info!(contract = contract.to_string(), "Migrated contract");
            Ok(())
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
) -> AppResult<()> {
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
    account.code_hash = new_code_hash;
    ACCOUNTS.save(&mut store, contract, &account)?;

    // load wasm code
    let wasm_byte_code = CODES.load(&store, &account.code_hash)?;

    // create wasm host
    let substore = PrefixStore::new(store.clone(), &[CONTRACT_NAMESPACE, &contract]);
    let querier = Querier::new(store, block.clone());
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

    debug_assert!(resp.msgs.is_empty(), "UNIMPLEMENTED: submessage is not supported yet");

    Ok(())
}
