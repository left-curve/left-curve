#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, CONTRACTS, EventResult, GasTracker, TraceOption, Vm,
        call_in_2_out_1_handle_response, catch_and_update_event, catch_event,
    },
    grug_types::{
        Addr, BlockInfo, Config, Context, EvtReply, Json, ReplyOn, Storage, SubMsgResult,
    },
};

pub fn do_reply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    msg_depth: usize,
    contract: Addr,
    msg: &Json,
    result: SubMsgResult,
    reply_on: &ReplyOn,
    trace_opt: TraceOption,
) -> EventResult<EvtReply>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_reply(
        vm,
        storage,
        gas_tracker,
        block,
        chain_id,
        cfg,
        app_cfg,
        msg_depth,
        contract,
        msg,
        result,
        reply_on,
        trace_opt,
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            dyn_event!(
                trace_opt.ok_level.into(),
                contract = contract.to_string(),
                "Performed reply"
            );
        },
        "Failed to perform reply",
        trace_opt.error_level.into(),
    );

    evt
}

fn _do_reply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    chain_id: String,
    cfg: &Config,
    app_cfg: Json,
    msg_depth: usize,
    contract: Addr,
    msg: &Json,
    result: SubMsgResult,
    reply_on: &ReplyOn,
    trace_opt: TraceOption,
) -> EventResult<EvtReply>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtReply::base(contract, reply_on.clone());

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
        call_in_2_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            cfg,
            app_cfg,
            msg_depth,
            0,
            true,
            "reply",
            code_hash,
            &ctx,
            msg,
            &result,
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
