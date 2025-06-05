#[cfg(feature = "tracing")]
use {crate::TraceOption, dyn_event::dyn_event};
use {
    crate::{
        _do_transfer, AppError, CHAIN_ID, CONTRACTS, EventResult, GasTracker, Vm,
        call_in_1_out_1_handle_response, catch_and_update_event, catch_event,
    },
    grug_types::{Addr, BlockInfo, Context, EvtExecute, MsgExecute, Storage, btree_map},
};

pub fn do_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgExecute,
    trace_opt: TraceOption,
) -> EventResult<EvtExecute>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_execute(
        vm,
        storage,
        gas_tracker,
        block,
        msg_depth,
        sender,
        msg,
        trace_opt,
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |evt| {
            dyn_event!(
                trace_opt.ok_level,
                contract = evt.contract.to_string(),
                "Executed contract"
            );
        },
        "Failed to execute contract",
        trace_opt.error_level,
    );

    evt
}

fn _do_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgExecute,
    trace_opt: TraceOption,
) -> EventResult<EvtExecute>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtExecute::base(sender, msg.contract, msg.funds.clone(), msg.msg.clone());

    let (code_hash, chain_id) = catch_event! {
        {
            let code_hash = CONTRACTS.load(&storage, msg.contract)?.code_hash;
            let chain_id = CHAIN_ID.load(&storage)?;

            Ok((code_hash, chain_id))
        },
        evt
    };

    if !msg.funds.is_empty() {
        catch_and_update_event! {
            _do_transfer(
                vm.clone(),
                storage.clone(),
                gas_tracker.clone(),
                block,
                msg_depth,
                sender,
                btree_map! { msg.contract => msg.funds.clone() },
                false,
                trace_opt,
            ),
            evt => transfer_event
        }
    }

    // Call the contract's `execute` entry point
    let ctx = Context {
        chain_id,
        block,
        contract: msg.contract,
        sender: Some(sender),
        funds: Some(msg.funds.clone()),
        mode: None,
    };

    catch_and_update_event! {
        call_in_1_out_1_handle_response(
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
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
