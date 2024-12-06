use {
    crate::{
        call_in_0_out_1_handle_response, call_in_1_out_1, call_in_1_out_1_handle_response,
        call_in_2_out_1_handle_response, catch_event, handle_response, has_permission,
        schedule_cronjob, update_event_field, AppCtx, AppError, AppResult, EventResult,
        MeteredItem, MeteredMap, Vm, APP_CONFIG, CODES, CONFIG, CONTRACTS, NEXT_CRONJOBS,
    },
    grug_math::Inner,
    grug_types::{
        Addr, AuthMode, AuthResponse, BankMsg, Code, CodeStatus, Context, ContractInfo,
        EventStatus, EvtAuthenticate, EvtBackrun, EvtConfigure, EvtExecute, EvtFinalize, EvtGuest,
        EvtInstantiate, EvtMigrate, EvtReply, EvtTransfer, EvtUpload, EvtWithhold, GenericResult,
        Hash256, HashExt, Json, MsgConfigure, MsgExecute, MsgInstantiate, MsgMigrate, MsgTransfer,
        MsgUpload, ReplyOn, StdResult, SubMsgResult, Tx, TxOutcome,
    },
};

// ---------------------------------- config -----------------------------------

pub fn do_configure(ctx: AppCtx, sender: Addr, msg: MsgConfigure) -> EventResult<EvtConfigure> {
    let evt = EvtConfigure { sender };

    match _do_configure(ctx, sender, msg) {
        Ok(_) => {
            #[cfg(feature = "tracing")]
            tracing::info!("Config updated");

            EventResult::Ok(evt)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to updated config");

            EventResult::err(evt, err)
        },
    }
}

fn _do_configure(mut ctx: AppCtx, sender: Addr, msg: MsgConfigure) -> AppResult<()> {
    let cfg = CONFIG.load(&ctx.storage)?;

    // Make sure the sender is authorized to set the config.
    if sender != cfg.owner {
        return Err(AppError::NotOwner {
            sender,
            owner: cfg.owner,
        });
    }

    if let Some(new_cfg) = msg.new_cfg {
        // If the list of cronjobs has been changed, we have to delete the
        // existing scheduled ones and reschedule.
        if new_cfg.cronjobs != cfg.cronjobs {
            NEXT_CRONJOBS.clear(&mut ctx.storage, None, None);

            for (contract, interval) in &new_cfg.cronjobs {
                schedule_cronjob(&mut ctx.storage, *contract, ctx.block.timestamp, *interval)?;
            }
        }

        CONFIG.save(&mut ctx.storage, &new_cfg)?;
    }

    if let Some(new_app_cfg) = msg.new_app_cfg {
        APP_CONFIG.save(&mut ctx.storage, &new_app_cfg)?;
    }

    Ok(())
}

// ---------------------------------- upload -----------------------------------

pub fn do_upload(ctx: AppCtx, uploader: Addr, msg: MsgUpload) -> EventResult<EvtUpload> {
    let code_hash = msg.code.hash256();

    let evt = EvtUpload {
        sender: uploader,
        code_hash,
    };

    match _do_upload(ctx, uploader, msg, code_hash) {
        Ok(_) => {
            #[cfg(feature = "tracing")]
            tracing::info!(code_hash = code_hash.to_string(), "Uploaded code");

            EventResult::Ok(evt)
        },
        Err(err) => {
            #[cfg(feature = "tracing")]
            tracing::warn!(err = err.to_string(), "Failed to upload code");

            EventResult::err(evt, err)
        },
    }
}

// Return the hash of the code that is stored, for logging purpose.
fn _do_upload(
    mut ctx: AppCtx,
    uploader: Addr,
    msg: MsgUpload,
    code_hash: Hash256,
) -> AppResult<()> {
    // Make sure the user has the permission to upload contracts
    let cfg = CONFIG.load_with_gas(&ctx.storage, ctx.gas_tracker.clone())?;

    if !has_permission(&cfg.permissions.upload, cfg.owner, uploader) {
        return Err(AppError::Unauthorized);
    }

    if CODES.has_with_gas(&ctx.storage, ctx.gas_tracker.clone(), code_hash)? {
        return Err(AppError::CodeExists { code_hash });
    }

    CODES.save_with_gas(&mut ctx.storage, ctx.gas_tracker, code_hash, &Code {
        code: msg.code,
        status: CodeStatus::Orphaned {
            since: ctx.block.timestamp,
        },
    })?;

    Ok(())
}

// --------------------------------- transfer ----------------------------------

pub fn do_transfer<VM>(
    ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgTransfer,
    do_receive: bool,
) -> EventResult<EvtTransfer>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_transfer(ctx, msg_depth, sender, msg.clone(), do_receive);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(
                from = sender.to_string(),
                to = msg.to.to_string(),
                coins = msg.coins.to_string(),
                "Transferred coins"
            );
        },
        "Failed to transfer coins",
    );

    evt
}

fn _do_transfer<VM>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgTransfer,
    // Whether to call the receipient account's `receive` entry point following
    // the transfer, to inform it that the transfer has happened.
    // - `true` when handling `Message::Transfer`
    // - `false` when handling `Message::{Instantaite,Execute}`
    do_receive: bool,
) -> EventResult<EvtTransfer>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtTransfer::base(sender, msg.to, msg.coins.clone());

    let (cfg, code_hash) = catch_event!(evt, {
        let cfg = CONFIG.load(&app_ctx.storage)?;
        let code_hash = CONTRACTS.load(&app_ctx.storage, cfg.bank)?.code_hash;
        Ok((cfg, code_hash))
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: cfg.bank,
        sender: None,
        funds: None,
        mode: None,
    };

    let msg = BankMsg {
        from: sender,
        to: msg.to,
        coins: msg.coins.clone(),
    };

    let bank_guest = call_in_1_out_1_handle_response(
        app_ctx.clone(),
        msg_depth,
        0,
        true,
        "bank_execute",
        code_hash,
        &ctx,
        &msg,
    );

    update_event_field!(bank_guest, evt => bank_guest);

    if do_receive {
        let receive_guest = _do_receive(app_ctx, msg_depth, msg.clone());
        update_event_field!(receive_guest, evt => receive_guest);
    };

    EventResult::Ok(evt)
}

fn _do_receive<VM>(app_ctx: AppCtx<VM>, msg_depth: usize, msg: BankMsg) -> EventResult<EvtGuest>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let code_hash = catch_event!(EvtGuest::base(msg.to, "receive"), {
        Ok(CONTRACTS.load(&app_ctx.storage, msg.to)?.code_hash)
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: msg.to,
        sender: Some(msg.from),
        funds: Some(msg.coins),
        mode: None,
    };

    call_in_0_out_1_handle_response(app_ctx, msg_depth, 0, true, "receive", code_hash, &ctx)
}

// -------------------------------- instantiate --------------------------------

pub fn do_instantiate<VM>(
    ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgInstantiate,
) -> EventResult<EvtInstantiate>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_instantiate(ctx, msg_depth, sender, msg.clone());

    evt.debug(
        |evt| {
            #[cfg(feature = "tracing")]
            tracing::info!(address = evt.contract.to_string(), "Instantiated contract");
        },
        "Failed to instantiate contract",
    );

    evt
}

pub fn _do_instantiate<VM>(
    mut app_ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgInstantiate,
) -> EventResult<EvtInstantiate>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    // Compute the contract address, and make sure there isn't already a
    // contract of the same address.
    let address = Addr::derive(sender, msg.code_hash, &msg.salt);

    let mut evt = EvtInstantiate::base(sender, msg.code_hash, address);

    catch_event!(evt, {
        // Make sure the user has the permission to instantiate contracts
        let cfg = CONFIG.load(&app_ctx.storage)?;

        if !has_permission(&cfg.permissions.instantiate, cfg.owner, sender) {
            return Err(AppError::Unauthorized);
        }

        // Save the contract info
        CONTRACTS.may_update(&mut app_ctx.storage, address, |maybe_contract| {
            if maybe_contract.is_some() {
                return Err(AppError::AccountExists { address });
            }

            Ok(ContractInfo {
                code_hash: msg.code_hash,
                label: msg.label.clone().map(Inner::into_inner),
                admin: msg.admin,
            })
        })?;

        // Increment the code's usage.
        CODES.update(
            &mut app_ctx.storage,
            msg.code_hash,
            |mut code| -> StdResult<_> {
                match &mut code.status {
                    CodeStatus::Orphaned { .. } => {
                        code.status = CodeStatus::InUse { usage: 1 };
                    },
                    CodeStatus::InUse { usage } => {
                        *usage += 1;
                    },
                }

                Ok(code)
            },
        )?;

        Ok(())
    });

    if msg.funds.is_empty() {
        let transfer_event = _do_transfer(
            app_ctx.clone(),
            msg_depth,
            sender,
            MsgTransfer {
                to: address,
                coins: msg.funds.clone(),
            },
            false,
        );

        update_event_field!(transfer_event, evt => transfer_event);
    }

    // Call the contract's `instantiate` entry point
    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: address,
        sender: Some(sender),
        funds: Some(msg.funds),
        mode: None,
    };

    let guest_event = call_in_1_out_1_handle_response(
        app_ctx,
        msg_depth,
        0,
        true,
        "instantiate",
        msg.code_hash,
        &ctx,
        &msg.msg,
    );

    update_event_field!(guest_event, evt => guest_event);

    EventResult::Ok(evt)
}

// ---------------------------------- execute ----------------------------------

pub fn do_execute<VM>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgExecute,
) -> EventResult<EvtExecute>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_execute(app_ctx, msg_depth, sender, msg.clone());

    evt.debug(
        |evt| {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = evt.contract.to_string(), "Executed contract");
        },
        "Failed to execute contract",
    );

    evt
}

fn _do_execute<VM>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgExecute,
) -> EventResult<EvtExecute>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtExecute::base(sender, msg.contract, msg.funds.clone());

    let code_hash = catch_event!(evt, {
        Ok(CONTRACTS.load(&app_ctx.storage, msg.contract)?.code_hash)
    });

    if !msg.funds.is_empty() {
        let transfer_event = _do_transfer(
            app_ctx.clone(),
            msg_depth,
            sender,
            MsgTransfer {
                to: msg.contract,
                coins: msg.funds.clone(),
            },
            false,
        );

        update_event_field!(transfer_event, evt => transfer_event);
    }

    // Call the contract's `execute` entry point
    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: msg.contract,
        sender: Some(sender),
        funds: Some(msg.funds.clone()),
        mode: None,
    };

    let guest_event = call_in_1_out_1_handle_response(
        app_ctx, msg_depth, 0, true, "execute", code_hash, &ctx, &msg.msg,
    );

    update_event_field!(guest_event, evt => guest_event);

    EventResult::Ok(evt)
}

// ---------------------------------- migrate ----------------------------------

pub fn do_migrate<VM>(
    ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgMigrate,
) -> EventResult<EvtMigrate>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_migrate(ctx, msg_depth, sender, msg);

    evt.debug(
        |evt| {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = evt.contract.to_string(), "Migrated contract");
        },
        "Failed to migrate contract",
    );

    evt
}

fn _do_migrate<VM>(
    mut app_ctx: AppCtx<VM>,
    msg_depth: usize,
    sender: Addr,
    msg: MsgMigrate,
) -> EventResult<EvtMigrate>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtMigrate::base(sender, msg.contract, msg.new_code_hash);

    let old_code_hash = catch_event!(evt, {
        // Update the contract info.
        let mut old_code_hash = None;
        CONTRACTS.update(&mut app_ctx.storage, msg.contract, |mut info| {
            old_code_hash = Some(info.code_hash);

            // Ensure the sender is the admin of the contract.
            if Some(sender) != info.admin {
                return Err(AppError::Unauthorized);
            }

            info.code_hash = msg.new_code_hash;

            Ok(info)
        })?;

        let old_code_hash = old_code_hash.unwrap();

        // Reduce usage count of the old code.
        CODES.update(
            &mut app_ctx.storage,
            old_code_hash,
            |mut code| -> StdResult<_> {
                match &mut code.status {
                    CodeStatus::InUse { usage } => {
                        if *usage == 1 {
                            code.status = CodeStatus::Orphaned {
                                since: app_ctx.block.timestamp,
                            };
                        } else {
                            *usage -= 1;
                        }
                    },
                    _ => unreachable!(),
                }

                Ok(code)
            },
        )?;

        // Increase usage count of the new code.
        CODES.update(
            &mut app_ctx.storage,
            msg.new_code_hash,
            |mut code| -> StdResult<_> {
                match &mut code.status {
                    CodeStatus::Orphaned { .. } => {
                        code.status = CodeStatus::InUse { usage: 1 };
                    },
                    CodeStatus::InUse { usage } => {
                        *usage += 1;
                    },
                }

                Ok(code)
            },
        )?;

        Ok(old_code_hash)
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: msg.contract,
        sender: None,
        funds: None,
        mode: None,
    };

    evt.old_code_hash = Some(old_code_hash);

    let guest_event = call_in_1_out_1_handle_response(
        app_ctx,
        msg_depth,
        0,
        true,
        "migrate",
        msg.new_code_hash,
        &ctx,
        &msg.msg,
    );

    update_event_field!(guest_event, evt => guest_event);

    EventResult::Ok(evt)
}

// ----------------------------------- reply -----------------------------------

pub fn do_reply<VM>(
    ctx: AppCtx<VM>,
    msg_depth: usize,
    contract: Addr,
    msg: &Json,
    result: &SubMsgResult,
    reply_on: &ReplyOn,
) -> EventResult<EvtReply>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_reply(ctx, msg_depth, contract, msg, result, reply_on);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = contract.to_string(), "Performed reply");
        },
        "Failed to perform reply",
    );

    evt
}

fn _do_reply<VM>(
    app_ctx: AppCtx<VM>,
    msg_depth: usize,
    contract: Addr,
    msg: &Json,
    result: &SubMsgResult,
    reply_on: &ReplyOn,
) -> EventResult<EvtReply>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtReply::base(contract, reply_on);

    let code_hash = catch_event!(evt, {
        Ok(CONTRACTS.load(&app_ctx.storage, contract)?.code_hash)
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract,
        sender: None,
        funds: None,
        mode: None,
    };

    let guest_event = call_in_2_out_1_handle_response(
        app_ctx, msg_depth, 0, true, "reply", code_hash, &ctx, msg, result,
    );

    update_event_field!(guest_event, evt => guest_event);

    EventResult::Ok(evt)
}

// ------------------------------- authenticate --------------------------------

pub fn do_authenticate<VM>(
    app_ctx: AppCtx<VM>,
    tx: &Tx,
    mode: AuthMode,
) -> EventResult<EvtAuthenticate>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_authenticate(app_ctx, tx, mode);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(sender = tx.sender.to_string(), "Authenticated transaction");
        },
        "Failed to authenticate transaction",
    );

    evt
}

pub fn _do_authenticate<VM>(
    app_ctx: AppCtx<VM>,
    tx: &Tx,
    mode: AuthMode,
) -> EventResult<EvtAuthenticate>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtAuthenticate::base(tx.sender);

    let code_hash = catch_event!(evt, {
        Ok(CONTRACTS.load(&app_ctx.storage, tx.sender)?.code_hash)
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: tx.sender,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    let result = || -> EventResult<EvtGuest> {
        let evt = EvtGuest::base(ctx.contract, "authenticate");

        let auth_response = catch_event!(evt, {
            call_in_1_out_1::<_, _, GenericResult<AuthResponse>>(
                app_ctx.clone(),
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
            })
        });

        handle_response(app_ctx, 0, &ctx, auth_response.response, evt)
    }();

    update_event_field!(result, evt => guest_event);

    EventResult::Ok(evt)
}

// ---------------------------------- backrun ----------------------------------

pub fn do_backrun<VM>(app_ctx: AppCtx<VM>, tx: &Tx, mode: AuthMode) -> EventResult<EvtBackrun>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_backrun(app_ctx, tx, mode);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(sender = tx.sender.to_string(), "Backran transaction");
        },
        "Failed to backrun transaction",
    );

    evt
}

pub fn _do_backrun<VM>(app_ctx: AppCtx<VM>, tx: &Tx, mode: AuthMode) -> EventResult<EvtBackrun>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtBackrun::base(tx.sender);

    let code_hash = catch_event!(evt, {
        Ok(CONTRACTS.load(&app_ctx.storage, tx.sender)?.code_hash)
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: tx.sender,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    let guest_evt =
        call_in_1_out_1_handle_response(app_ctx, 0, 0, true, "backrun", code_hash, &ctx, tx);

    update_event_field!(guest_evt, evt => guest_event);

    EventResult::Ok(evt)
}

// ---------------------------------- taxman -----------------------------------

pub fn do_withhold_fee<VM>(app_ctx: AppCtx<VM>, tx: &Tx, mode: AuthMode) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_withhold_fee(app_ctx, tx, mode);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(sender = tx.sender.to_string(), "Withheld fee");
        },
        "Failed to withhold fee",
    );

    evt
}

pub fn _do_withhold_fee<VM>(
    app_ctx: AppCtx<VM>,
    tx: &Tx,
    mode: AuthMode,
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtWithhold::base(tx.sender);

    let (cfg, taxman) = catch_event!(evt, {
        let cfg = CONFIG.load(&app_ctx.storage)?;
        let taxman = CONTRACTS.load(&app_ctx.storage, cfg.taxman)?;

        Ok((cfg, taxman))
    });

    evt.taxman = Some(cfg.taxman);

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: cfg.taxman,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    let guest_event = call_in_1_out_1_handle_response(
        app_ctx,
        0,
        0,
        true,
        "withhold_fee",
        taxman.code_hash,
        &ctx,
        tx,
    );

    update_event_field!(guest_event, evt => guest_event);

    EventResult::Ok(evt)
}

pub fn do_finalize_fee<VM>(
    app_ctx: AppCtx<VM>,
    tx: &Tx,
    outcome: &TxOutcome,
    mode: AuthMode,
) -> EventResult<EvtFinalize>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_finalize_fee(app_ctx, tx, outcome, mode);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(sender = tx.sender.to_string(), "Finalized fee");
        },
        "Failed to finalize fee",
        // `finalize_fee` is supposed to always succeed, so if it doesn't,
        // we print a tracing log at ERROR level to highlight the seriousness.
        // #[cfg(feature = "tracing")]
        // tracing::error!(err = err.to_string(), "Failed to finalize fee")
    );

    evt
}

pub fn _do_finalize_fee<VM>(
    app_ctx: AppCtx<VM>,
    tx: &Tx,
    outcome: &TxOutcome,
    mode: AuthMode,
) -> EventResult<EvtFinalize>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let mut evt = EvtFinalize::base(tx.sender);

    let (cfg, taxman) = catch_event!(evt, {
        let cfg = CONFIG.load(&app_ctx.storage)?;
        let taxman = CONTRACTS.load(&app_ctx.storage, cfg.taxman)?;

        Ok((cfg, taxman))
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract: cfg.taxman,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    let evt_guest = call_in_2_out_1_handle_response(
        app_ctx,
        0,
        0,
        true,
        "finalize_fee",
        taxman.code_hash,
        &ctx,
        tx,
        outcome,
    );

    update_event_field!(evt_guest, evt => guest_event);

    EventResult::Ok(evt)
}

// ----------------------------------- cron ------------------------------------

pub fn do_cron_execute<VM>(ctx: AppCtx<VM>, contract: Addr) -> EventResult<EvtGuest>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_cron_execute(ctx, contract);

    evt.debug(
        |_| {
            #[cfg(feature = "tracing")]
            tracing::info!(contract = contract.to_string(), "Performed cronjob");
        },
        "Failed to perform cronjob",
    );

    evt
}

fn _do_cron_execute<VM>(app_ctx: AppCtx<VM>, contract: Addr) -> EventResult<EvtGuest>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let code_hash = catch_event!(EvtGuest::base(contract, "cron_execute"), {
        Ok(CONTRACTS.load(&app_ctx.storage, contract)?.code_hash)
    });

    let ctx = Context {
        chain_id: app_ctx.chain_id.clone(),
        block: app_ctx.block,
        contract,
        sender: None,
        funds: None,
        mode: None,
    };

    call_in_0_out_1_handle_response(app_ctx, 0, 0, true, "cron_execute", code_hash, &ctx)
}
