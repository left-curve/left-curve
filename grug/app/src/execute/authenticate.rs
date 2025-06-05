#[cfg(feature = "tracing")]
use {crate::TraceOption, dyn_event::dyn_event};
use {
    crate::{
        AppError, CHAIN_ID, CONTRACTS, EventResult, GasTracker, Vm,
        call_in_1_out_1_handle_auth_response, catch_and_update_event, catch_event,
    },
    grug_types::{AuthMode, BlockInfo, Context, EvtAuthenticate, Storage, Tx},
};

pub fn do_authenticate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
    trace_opt: TraceOption,
) -> EventResult<EvtAuthenticate>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_authenticate(vm, storage, gas_tracker, block, tx, mode, trace_opt);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            dyn_event!(
                trace_opt.ok_level.into(),
                sender = tx.sender.to_string(),
                "Authenticated transaction"
            );
        },
        "Failed to authenticate transaction",
        trace_opt.error_level.into(),
    );

    evt
}

pub fn _do_authenticate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
    trace_opt: TraceOption,
) -> EventResult<EvtAuthenticate>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtAuthenticate::base(tx.sender);

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
        call_in_1_out_1_handle_auth_response(
            vm,
            storage,
            gas_tracker,
            0,
            0,
            true,
            "authenticate",
            code_hash,
            &ctx,
            tx,
            &mut evt.backrun,
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
