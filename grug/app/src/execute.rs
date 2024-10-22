use {
    crate::{
        call_in_0_out_1_handle_response, call_in_1_out_1, call_in_1_out_1_handle_response,
        call_in_2_out_1_handle_response, handle_response, has_permission, schedule_cronjob,
        AppError, AppResult, GasTracker, MeteredItem, MeteredMap, Vm, APP_CONFIGS, CHAIN_ID, CODES,
        CONFIG, CONTRACTS, NEXT_CRONJOBS,
    },
    grug_math::Inner,
    grug_types::{
        Addr, AuthMode, AuthResponse, BankMsg, BlockInfo, Code, CodeStatus, Context, ContractInfo,
        Event, GenericResult, Hash256, HashExt, Json, MsgConfigure, MsgExecute, MsgInstantiate,
        MsgMigrate, MsgTransfer, MsgUpload, Op, Storage, SubMsgResult, Tx, TxOutcome,
    },
};

// ---------------------------------- config -----------------------------------

pub fn do_configure(
    storage: &mut dyn Storage,
    block: BlockInfo,
    sender: Addr,
    msg: MsgConfigure,
) -> AppResult<Vec<Event>> {
    match _do_configure(storage, block, sender, msg) {
        Ok(event) => {
            #[cfg(feature = "tracing")]
            tracing::info!("Config updated");

            Ok(vec![event])
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to updated config");

            Err(err)
        },
    }
}

fn _do_configure(
    storage: &mut dyn Storage,
    block: BlockInfo,
    sender: Addr,
    msg: MsgConfigure,
) -> AppResult<Event> {
    let mut cfg = CONFIG.load(storage)?;

    // Make sure the sender is authorized to set the config.
    if sender != cfg.owner {
        return Err(AppError::NotOwner {
            sender,
            owner: cfg.owner,
        });
    }

    if let Some(new_owner) = msg.updates.owner {
        cfg.owner = new_owner;
    }

    if let Some(new_bank) = msg.updates.bank {
        cfg.bank = new_bank;
    }

    if let Some(new_taxman) = msg.updates.taxman {
        cfg.taxman = new_taxman;
    }

    if let Some(new_cronjobs) = msg.updates.cronjobs {
        // If the list of cronjobs has been changed, we have to delete the
        // existing scheduled ones and reschedule.
        if new_cronjobs != cfg.cronjobs {
            NEXT_CRONJOBS.clear(storage, None, None);

            for (contract, interval) in &new_cronjobs {
                schedule_cronjob(storage, *contract, block.timestamp, *interval)?;
            }
        }

        cfg.cronjobs = new_cronjobs;
    }

    if let Some(new_permissions) = msg.updates.permissions {
        cfg.permissions = new_permissions;
    }

    // Save the updated config.
    CONFIG.save(storage, &cfg)?;

    // Update app configs
    for (key, op) in msg.app_updates {
        if let Op::Insert(value) = op {
            APP_CONFIGS.save(storage, &key, &value)?;
        } else {
            APP_CONFIGS.remove(storage, &key);
        }
    }

    Ok(Event::new("configure").add_attribute("sender", sender))
}

// ---------------------------------- upload -----------------------------------

pub fn do_upload(
    storage: &mut dyn Storage,
    gas_tracker: GasTracker,
    block: BlockInfo,
    uploader: Addr,
    msg: MsgUpload,
) -> AppResult<Vec<Event>> {
    match _do_upload(storage, gas_tracker, block, uploader, msg) {
        Ok((event, _code_hash)) => {
            #[cfg(feature = "tracing")]
            tracing::info!(code_hash = _code_hash.to_string(), "Uploaded code");

            Ok(vec![event])
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to upload code");

            Err(err)
        },
    }
}

// Return the hash of the code that is stored, for logging purpose.
fn _do_upload(
    storage: &mut dyn Storage,
    gas_tracker: GasTracker,
    block: BlockInfo,
    uploader: Addr,
    msg: MsgUpload,
) -> AppResult<(Event, Hash256)> {
    // Make sure the user has the permission to upload contracts
    let cfg = CONFIG.load_with_gas(storage, gas_tracker.clone())?;

    if !has_permission(&cfg.permissions.upload, cfg.owner, uploader) {
        return Err(AppError::Unauthorized);
    }

    // Make sure that the same code isn't already uploaded
    let code_hash = msg.code.hash256();

    if CODES.has_with_gas(storage, gas_tracker.clone(), code_hash)? {
        return Err(AppError::CodeExists { code_hash });
    }

    CODES.save_with_gas(storage, gas_tracker, code_hash, &Code {
        code: msg.code,
        status: CodeStatus::Orphaned {
            since: block.timestamp,
        },
    })?;

    Ok((
        Event::new("upload").add_attribute("code_hash", code_hash),
        code_hash,
    ))
}

// --------------------------------- transfer ----------------------------------

pub fn do_transfer<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgTransfer,
    do_receive: bool,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match _do_transfer(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        block,
        sender,
        msg.clone(),
        do_receive,
    ) {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::info!(
                from = sender.to_string(),
                to = msg.to.to_string(),
                coins = msg.coins.to_string(),
                "Transferred coins"
            );

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to transfer coins");

            Err(err)
        },
    }
}

fn _do_transfer<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgTransfer,
    // Whether to call the receipient account's `receive` entry point following
    // the transfer, to inform it that the transfer has happened.
    // - `true` when handling `Message::Transfer`
    // - `false` when handling `Message::{Instantaite,Execute}`
    do_receive: bool,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let cfg = CONFIG.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, cfg.bank)?.code_hash;

    let ctx = Context {
        chain_id,
        block,
        contract: cfg.bank,
        sender: None,
        funds: None,
        mode: None,
    };

    let msg = BankMsg {
        from: sender,
        to: msg.to,
        coins: msg.coins,
    };

    let mut events = call_in_1_out_1_handle_response(
        vm.clone(),
        storage.clone(),
        gas_tracker.clone(),
        msg_depth,
        0,
        true,
        "bank_execute",
        code_hash,
        &ctx,
        &msg,
    )?;

    if do_receive {
        events.extend(_do_receive(
            vm,
            storage,
            gas_tracker,
            msg_depth,
            ctx.block,
            msg,
        )?);
    }

    Ok(events)
}

fn _do_receive<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    msg: BankMsg,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, msg.to)?.code_hash;

    let ctx = Context {
        chain_id,
        block,
        contract: msg.to,
        sender: Some(msg.from),
        funds: Some(msg.coins),
        mode: None,
    };

    call_in_0_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        0,
        true,
        "receive",
        code_hash,
        &ctx,
    )
}

// -------------------------------- instantiate --------------------------------

pub fn do_instantiate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgInstantiate,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match _do_instantiate(vm, storage, gas_tracker, msg_depth, block, sender, msg) {
        Ok((events, _address)) => {
            #[cfg(feature = "tracing")]
            tracing::info!(address = _address.to_string(), "Instantiated contract");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to instantiate contract");

            Err(err)
        },
    }
}

pub fn _do_instantiate<VM>(
    vm: VM,
    mut storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgInstantiate,
) -> AppResult<(Vec<Event>, Addr)>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;

    // Make sure the user has the permission to instantiate contracts
    let cfg = CONFIG.load(&storage)?;

    if !has_permission(&cfg.permissions.instantiate, cfg.owner, sender) {
        return Err(AppError::Unauthorized);
    }

    // Compute the contract address, and make sure there isn't already a
    // contract of the same address.
    let address = Addr::derive(sender, msg.code_hash, &msg.salt);

    if CONTRACTS.has(&storage, address) {
        return Err(AppError::AccountExists { address });
    }

    // Save the contract info
    let contract = ContractInfo {
        code_hash: msg.code_hash,
        label: msg.label.map(Inner::into_inner),
        admin: msg.admin,
    };

    CONTRACTS.save(&mut storage, address, &contract)?;

    // Increment the code's usage.
    let mut code = CODES.load(&storage, msg.code_hash)?;

    match &mut code.status {
        CodeStatus::Orphaned { .. } => code.status = CodeStatus::InUse { usage: 1 },
        CodeStatus::InUse { usage } => *usage += 1,
    }

    CODES.save(&mut storage, msg.code_hash, &code)?;

    // Make the fund transfer
    let mut events = vec![];

    if !msg.funds.is_empty() {
        events.extend(_do_transfer(
            vm.clone(),
            storage.clone(),
            gas_tracker.clone(),
            msg_depth,
            block,
            sender,
            MsgTransfer {
                to: address,
                coins: msg.funds.clone(),
            },
            false,
        )?);
    }

    // Call the contract's `instantiate` entry point
    let ctx = Context {
        chain_id,
        block,
        contract: address,
        sender: Some(sender),
        funds: Some(msg.funds),
        mode: None,
    };

    events.extend(call_in_1_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        0,
        true,
        "instantiate",
        contract.code_hash,
        &ctx,
        &msg.msg,
    )?);

    Ok((events, ctx.contract))
}

// ---------------------------------- execute ----------------------------------

pub fn do_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgExecute,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match _do_execute(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        block,
        sender,
        msg.clone(),
    ) {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = msg.contract.to_string(), "Executed contract");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to execute contract");

            Err(err)
        },
    }
}

fn _do_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgExecute,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, msg.contract)?.code_hash;

    // Make the fund transfer
    let mut events = vec![];

    if !msg.funds.is_empty() {
        events.extend(_do_transfer(
            vm.clone(),
            storage.clone(),
            gas_tracker.clone(),
            msg_depth,
            block,
            sender,
            MsgTransfer {
                to: msg.contract,
                coins: msg.funds.clone(),
            },
            false,
        )?);
    }

    // Call the contract's `execute` entry point
    let ctx = Context {
        chain_id,
        block,
        contract: msg.contract,
        sender: Some(sender),
        funds: Some(msg.funds),
        mode: None,
    };

    events.extend(call_in_1_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        0,
        true,
        "execute",
        code_hash,
        &ctx,
        &msg.msg,
    )?);

    Ok(events)
}

// ---------------------------------- migrate ----------------------------------

pub fn do_migrate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgMigrate,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match _do_migrate(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        block,
        sender,
        msg.clone(),
    ) {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = msg.contract.to_string(), "Migrated contract");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to migrate contract");

            Err(err)
        },
    }
}

fn _do_migrate<VM>(
    vm: VM,
    mut storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    sender: Addr,
    msg: MsgMigrate,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let mut contract_info = CONTRACTS.load(&storage, msg.contract)?;

    // Only the account's admin can migrate it
    let Some(admin) = &contract_info.admin else {
        return Err(AppError::AdminNotSet);
    };

    if sender != admin {
        return Err(AppError::NotAdmin {
            sender,
            admin: contract_info.admin.unwrap(),
        });
    }

    let old_code_hash = contract_info.code_hash;

    // Update account info and save
    contract_info.code_hash = msg.new_code_hash;

    CONTRACTS.save(&mut storage, msg.contract, &contract_info)?;

    let mut old_code = CODES.load(&storage, old_code_hash)?;

    // Reduce the code's usage count.
    if let CodeStatus::InUse { usage: amount } = &mut old_code.status {
        *amount -= 1;

        if amount == &0 {
            old_code.status = CodeStatus::Orphaned {
                since: block.timestamp,
            };
        }

        CODES.save(&mut storage, old_code_hash, &old_code)?;
    }

    let mut new_code = CODES.load(&storage, msg.new_code_hash)?;

    match &mut new_code.status {
        CodeStatus::Orphaned { .. } => new_code.status = CodeStatus::InUse { usage: 1 },
        CodeStatus::InUse { usage: amount } => *amount += 1,
    }

    CODES.save(&mut storage, msg.new_code_hash, &new_code)?;

    let ctx = Context {
        chain_id,
        block,
        contract: msg.contract,
        sender: Some(sender),
        funds: None,
        mode: None,
    };

    call_in_1_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        0,
        true,
        "migrate",
        contract_info.code_hash,
        &ctx,
        &msg.msg,
    )
}

// ----------------------------------- reply -----------------------------------

pub fn do_reply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    contract: Addr,
    msg: &Json,
    result: &SubMsgResult,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match _do_reply(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        block,
        contract,
        msg,
        result,
    ) {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = contract.to_string(), "Performed reply");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to perform reply");

            Err(err)
        },
    }
}

fn _do_reply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    msg_depth: usize,
    block: BlockInfo,
    contract: Addr,
    msg: &Json,
    result: &SubMsgResult,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, contract)?.code_hash;

    let ctx = Context {
        chain_id,
        block,
        contract,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_2_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        msg_depth,
        0,
        true,
        "reply",
        code_hash,
        &ctx,
        msg,
        result,
    )
}

// ------------------------------- authenticate --------------------------------

pub fn do_authenticate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
) -> AppResult<(Vec<Event>, bool)>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, tx.sender)?.code_hash;

    let ctx = Context {
        chain_id,
        block,
        contract: tx.sender,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    let result = || -> AppResult<_> {
        let auth_response = call_in_1_out_1::<_, _, GenericResult<AuthResponse>>(
            vm.clone(),
            storage.clone(),
            gas_tracker.clone(),
            0,
            true,
            "authenticate",
            code_hash,
            &ctx,
            tx,
        )?
        .map_err(|msg| AppError::Guest {
            address: ctx.contract,
            name: "authenticate",
            msg,
        })?;

        let events = handle_response(
            vm,
            storage,
            gas_tracker,
            0,
            "authenticate",
            &ctx,
            auth_response.response,
        )?;

        Ok((events, auth_response.request_backrun))
    }();

    match result {
        Ok(data) => {
            #[cfg(feature = "tracing")]
            tracing::debug!(sender = tx.sender.to_string(), "Authenticated transaction");

            Ok(data)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to authenticate transaction");

            Err(err)
        },
    }
}

// ---------------------------------- backrun ----------------------------------

pub fn do_backrun<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, tx.sender)?.code_hash;

    let ctx = Context {
        chain_id,
        block,
        contract: tx.sender,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    match call_in_1_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        0,
        0,
        true,
        "backrun",
        code_hash,
        &ctx,
        tx,
    ) {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::debug!(sender = tx.sender.to_string(), "Backran transaction");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to backrun transaction");

            Err(err)
        },
    }
}

// ---------------------------------- taxman -----------------------------------

pub fn do_withhold_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let result = (|| {
        let chain_id = CHAIN_ID.load(&storage)?;
        let cfg = CONFIG.load(&storage)?;
        let taxman = CONTRACTS.load(&storage, cfg.taxman)?;

        let ctx = Context {
            chain_id,
            block,
            contract: cfg.taxman,
            sender: None,
            funds: None,
            mode: Some(mode),
        };

        call_in_1_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            0,
            0,
            true,
            "withhold_fee",
            taxman.code_hash,
            &ctx,
            tx,
        )
    })();

    match result {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::debug!(sender = tx.sender.to_string(), "Withheld fee");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to withhold fee");

            Err(err)
        },
    }
}

pub fn do_finalize_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    outcome: &TxOutcome,
    mode: AuthMode,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let result = (|| {
        let chain_id = CHAIN_ID.load(&storage)?;
        let cfg = CONFIG.load(&storage)?;
        let taxman = CONTRACTS.load(&storage, cfg.taxman)?;

        let ctx = Context {
            chain_id,
            block,
            contract: cfg.taxman,
            sender: None,
            funds: None,
            mode: Some(mode),
        };

        call_in_2_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            0,
            0,
            true,
            "finalize_fee",
            taxman.code_hash,
            &ctx,
            tx,
            outcome,
        )
    })();

    match result {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::debug!(sender = tx.sender.to_string(), "Finalized fee");

            Ok(events)
        },
        Err(err) => {
            // `finalize_fee` is supposed to always succeed, so if it doesn't,
            // we print a tracing log at ERROR level to highlight the seriousness.
            #[cfg(feature = "tracing")]
            tracing::error!(err = err.to_string(), "Failed to finalize fee");

            Err(err)
        },
    }
}

// ----------------------------------- cron ------------------------------------

pub fn do_cron_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    contract: Addr,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    match _do_cron_execute(vm, storage, gas_tracker, block, contract) {
        Ok(events) => {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = contract.to_string(), "Performed cronjob");

            Ok(events)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                contract = contract.to_string(),
                err = err.to_string(),
                "Failed to perform cronjob"
            );

            Err(err)
        },
    }
}

fn _do_cron_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    contract: Addr,
) -> AppResult<Vec<Event>>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let chain_id = CHAIN_ID.load(&storage)?;
    let code_hash = CONTRACTS.load(&storage, contract)?.code_hash;
    let ctx = Context {
        chain_id,
        block,
        contract,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_0_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        0,
        0,
        true,
        "cron_execute",
        code_hash,
        &ctx,
    )
}
