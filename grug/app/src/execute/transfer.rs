use {
    crate::{
        AppError, CHAIN_ID, CONFIG, CONTRACTS, EventResult, GasTracker, Vm,
        call_in_0_out_1_handle_response, call_in_1_out_1_handle_response, catch_and_insert_event,
        catch_and_update_event, catch_event,
    },
    grug_types::{
        Addr, BankMsg, BlockInfo, Coins, Context, EvtGuest, EvtTransfer, Hash256, MsgTransfer,
        Storage,
    },
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
    VM: Vm + Clone + 'static,
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
                transfers = ?msg,
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
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtTransfer::base(sender, msg.clone());

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
        transfers: msg,
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
        for (to, coins) in msg.transfers {
            // If recipient does not exist, skip the `_do_receive` call.
            if let Ok(Some(contract_info)) = CONTRACTS.may_load(&storage, to) {
                catch_and_insert_event! {
                    _do_receive(
                        vm.clone(),
                        storage.clone(),
                        gas_tracker.clone(),
                        block,
                        msg_depth,
                        msg.from,
                        to,
                        coins,
                        contract_info.code_hash,
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
    msg_depth: usize,
    from: Addr,
    to: Addr,
    coins: Coins,
    code_hash: Hash256,
) -> EventResult<EvtGuest>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    #[allow(clippy::redundant_closure_call)]
    let chain_id = catch_event! {
        {
            CHAIN_ID.load(&storage).map_err(Into::into)
        },
        EvtGuest::base(to, "receive")
    };

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
        msg_depth,
        0,
        true,
        "receive",
        code_hash,
        &ctx,
    )
}
