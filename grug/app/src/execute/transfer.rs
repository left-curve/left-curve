#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, CONTRACTS, EventResult, GasTracker, TraceOption, Vm,
        call_in_0_out_1_handle_response, call_in_1_out_1_handle_response, catch_and_insert_event,
        catch_and_update_event, catch_event,
    },
    grug_types::{
        Addr, BankMsg, BlockInfo, Coins, Config, Context, EvtGuest, EvtTransfer, Hash256, Json,
        MsgTransfer, Storage,
    },
};

pub fn do_transfer<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    msg_depth: usize,
    sender: Addr,
    msg: MsgTransfer,
    do_receive: bool,
    trace_opt: TraceOption,
) -> EventResult<EvtTransfer>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_transfer(
        vm,
        storage,
        gas_tracker,
        block,
        chain_id,
        cfg,
        app_cfg,
        msg_depth,
        sender,
        msg.clone(),
        do_receive,
        trace_opt,
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            dyn_event!(
                trace_opt.ok_level.into(),
                from = sender.to_string(),
                transfers = ?msg,
                "Transferred coins"
            );
        },
        "Failed to transfer coins",
        trace_opt.error_level.into(),
    );

    evt
}

pub(crate) fn _do_transfer<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    msg_depth: usize,
    sender: Addr,
    msg: MsgTransfer,
    // Whether to call the receipient account's `receive` entry point following
    // the transfer, to inform it that the transfer has happened.
    // - `true` when handling `Message::Transfer`
    // - `false` when handling `Message::{Instantaite,Execute}`
    do_receive: bool,
    trace_opt: TraceOption,
) -> EventResult<EvtTransfer>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtTransfer::base(sender, msg.clone());

    let code_hash = catch_event! {
        {
            let contract = CONTRACTS.load(&storage, cfg.bank)?;
            Ok::<_, AppError>(contract.code_hash)
        },
        evt
    };

    let ctx = Context {
        chain_id: chain_id.clone(),
        block,
        contract: cfg.bank,
        sender: None,
        funds: None,
        mode: None,
    };

    let msg = BankMsg {
        from: sender,
        transfers: msg,
    };

    catch_and_update_event! {
        call_in_1_out_1_handle_response(
            vm.clone(),
            storage.clone(),
            gas_tracker.clone(),
            cfg,
            app_cfg.clone(),
            msg_depth,
            0,
            true,
            "bank_execute",
            code_hash,
            &ctx,
            &msg,
            trace_opt,
        ),
        evt => bank_guest
    }

    if do_receive {
        for (to, coins) in msg.transfers {
            // If recipient does not exist, skip the `_do_receive` call.
            if let Ok(Some(contract_info)) = CONTRACTS.may_load(&storage, to) {
                catch_and_insert_event! {
                    _do_receive(
                        vm.clone(),
                        storage.clone(),
                        gas_tracker.clone(),
                        block,
                        chain_id.clone(),
                        cfg,
                        app_cfg.clone(),
                        msg_depth,
                        msg.from,
                        to,
                        coins,
                        contract_info.code_hash,
                        trace_opt,
                    ),
                    evt,
                    receive_guests,
                    key: to
                }
            }
        }
    };

    EventResult::Ok(evt)
}

fn _do_receive<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    msg_depth: usize,
    from: Addr,
    to: Addr,
    coins: Coins,
    code_hash: Hash256,
    trace_opt: TraceOption,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let ctx = Context {
        chain_id,
        block,
        contract: to,
        sender: Some(from),
        funds: Some(coins),
        mode: None,
    };

    call_in_0_out_1_handle_response(
        vm,
        storage,
        gas_tracker,
        cfg,
        app_cfg,
        msg_depth,
        0,
        true,
        "receive",
        code_hash,
        &ctx,
        trace_opt,
    )
}
