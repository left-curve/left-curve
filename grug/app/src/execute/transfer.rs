use {
    crate::{
        call_in_0_out_1_handle_response, call_in_1_out_1_handle_response, catch_and_update_event,
        catch_event, AppError, EventResult, GasTracker, Vm, CHAIN_ID, CONFIG, CONTRACTS,
    },
    grug_types::{Addr, BankMsg, BlockInfo, Context, EvtGuest, EvtTransfer, MsgTransfer, Storage},
};

pub fn do_transfer<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgTransfer,
    do_receive: bool,
) -> EventResult<EvtTransfer>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_transfer(
        vm,
        storage,
        gas_tracker,
        block,
        msg_depth,
        sender,
        msg.clone(),
        do_receive,
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
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

pub(crate) fn _do_transfer<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
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

    let (cfg, code_hash, chain_id) = catch_event! {
        {
            let cfg = CONFIG.load(&storage)?;
            let chain_id = CHAIN_ID.load(&storage)?;
            let code_hash = CONTRACTS.load(&storage, cfg.bank)?.code_hash;

            Ok((cfg, code_hash, chain_id))
        },
        evt
    };

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
        coins: msg.coins.clone(),
    };

    catch_and_update_event! {
        call_in_1_out_1_handle_response(
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
        ),
        evt => bank_guest
    }

    if do_receive {
        catch_and_update_event! {
            _do_receive(
                vm,
                storage,
                gas_tracker,
                block,
                msg_depth,
                msg,
            ),
            evt => receive_guest
        }
    };

    EventResult::Ok(evt)
}

fn _do_receive<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    msg: BankMsg,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let (code_hash, chain_id) = catch_event! {
        {
            let code_hash = CONTRACTS.load(&storage, msg.to)?.code_hash;
            let chain_id = CHAIN_ID.load(&storage)?;

            Ok((code_hash, chain_id))
        },
        EvtGuest::base(msg.to, "receive")
    };

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
