use {
    crate::{
        call_in_1_out_1_handle_response, catch_and_update_event, catch_event, AppError,
        EventResult, GasTracker, Vm, CHAIN_ID, CONTRACTS,
    },
    grug_types::{AuthMode, BlockInfo, Context, EvtBackrun, Storage, Tx},
};

pub fn do_backrun<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
) -> EventResult<EvtBackrun>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_backrun(vm, storage, gas_tracker, block, tx, mode);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            tracing::info!(sender = tx.sender.to_string(), "Backran transaction");
        },
        "Failed to backrun transaction",
    );

    evt
}

pub fn _do_backrun<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
) -> EventResult<EvtBackrun>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtBackrun::base(tx.sender);

    let (code_hash, chain_id) = catch_event! {
        {
            let code_hash = CONTRACTS.load(&storage, tx.sender)?.code_hash;
            let chain_id = CHAIN_ID.load(&storage)?;

            Ok((code_hash, chain_id))
        },
        evt
    };

    let ctx = Context {
        chain_id,
        block,
        contract: tx.sender,
        sender: None,
        funds: None,
        mode: Some(mode),
    };

    catch_and_update_event! {
        call_in_1_out_1_handle_response(
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
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
