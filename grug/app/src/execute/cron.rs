#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, CONTRACTS, EventResult, GasTracker, TraceOption, Vm,
        call_in_0_out_1_handle_response, catch_and_update_event, catch_event,
    },
    grug_types::{Addr, BlockInfo, Config, Context, EvtCron, Json, Storage, Timestamp},
};

pub fn do_cron_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    contract: Addr,
    time: Timestamp,
    next: Timestamp,
    trace_opt: TraceOption,
) -> EventResult<EvtCron>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_cron_execute(
        vm,
        storage,
        gas_tracker,
        block,
        chain_id,
        cfg,
        app_cfg,
        contract,
        time,
        next,
        trace_opt,
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            dyn_event!(
                trace_opt.ok_level.into(),
                contract = contract.to_string(),
                "Performed cronjob"
            );
        },
        "Failed to perform cronjob",
        trace_opt.error_level.into(),
    );

    evt
}

fn _do_cron_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    contract: Addr,
    time: Timestamp,
    next: Timestamp,
    trace_opt: TraceOption,
) -> EventResult<EvtCron>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtCron::base(contract, time, next);

    let code_hash = catch_event! {
        {
            let contract = CONTRACTS.load(&storage, contract)?;
            Ok(contract.code_hash)
        },
        evt
    };

    let ctx = Context {
        chain_id,
        block,
        contract,
        sender: None,
        funds: None,
        mode: None,
    };

    catch_and_update_event! {
        call_in_0_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            cfg,
            app_cfg,
            0,
            0,
            true,
            "cron_execute",
            code_hash,
            &ctx,
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
