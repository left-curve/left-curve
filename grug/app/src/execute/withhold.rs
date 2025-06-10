#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, CHAIN_ID, CONFIG, CONTRACTS, EventResult, GasTracker, TraceOption, Vm,
        call_in_1_out_1_handle_response, catch_and_update_event, catch_event,
    },
    grug_types::{AuthMode, BlockInfo, Context, EvtWithhold, Storage, Tx},
};

pub fn do_withhold_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
    trace_opt: TraceOption,
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_withhold_fee(vm, storage, gas_tracker, block, tx, mode, trace_opt);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            dyn_event!(
                trace_opt.ok_level.into(),
                sender = tx.sender.to_string(),
                "Withheld fee"
            );
        },
        "Failed to withhold fee",
        trace_opt.error_level.into(),
    );

    evt
}

pub fn _do_withhold_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
    trace_opt: TraceOption,
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtWithhold::base(tx.sender, tx.gas_limit);

    let (cfg, taxman, chain_id) = catch_event! {
        {
            let cfg = CONFIG.load(&storage)?;
            let chain_id = CHAIN_ID.load(&storage)?;
            let taxman = CONTRACTS.load(&storage, cfg.taxman)?;

            Ok((cfg, taxman, chain_id))
        },
        evt
    };

    evt.taxman = Some(cfg.taxman);

    let ctx = Context {
        chain_id,
        block,
        contract: cfg.taxman,
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
            "withhold_fee",
            taxman.code_hash,
            &ctx,
            tx,
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
