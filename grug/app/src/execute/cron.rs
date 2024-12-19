use {
    crate::{
        call_in_0_out_1_handle_response, catch_and_update_event, catch_event, AppError,
        EventResult, GasTracker, Vm, CHAIN_ID, CONTRACTS,
    },
    grug_types::{Addr, BlockInfo, Context, EvtCron, Storage, Timestamp},
};

pub fn do_cron_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    contract: Addr,
    time: Timestamp,
    next: Timestamp,
) -> EventResult<EvtCron>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_cron_execute(vm, storage, gas_tracker, block, contract, time, next);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            tracing::info!(contract = contract.to_string(), "Performed cronjob");
        },
        "Failed to perform cronjob",
    );

    evt
}

fn _do_cron_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,

    contract: Addr,
    time: Timestamp,
    next: Timestamp,
) -> EventResult<EvtCron>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtCron::base(contract, time, next);

    let (code_hash, chain_id) = catch_event! {
        {
            let code_hash = CONTRACTS.load(&storage, contract)?.code_hash;
            let chain_id = CHAIN_ID.load(&storage)?;

            Ok((code_hash, chain_id))
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
            0,
            0,
            true,
            "cron_execute",
            code_hash,
            &ctx,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
