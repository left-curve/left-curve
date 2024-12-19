use {
    crate::{
        call_in_2_out_1_handle_response, catch_and_update_event, catch_event, AppError,
        EventResult, GasTracker, Vm, CHAIN_ID, CONTRACTS,
    },
    grug_types::{Addr, BlockInfo, Context, EvtReply, Json, ReplyOn, Storage, SubMsgResult},
};

pub fn do_reply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    contract: Addr,
    msg: &Json,
    result: &SubMsgResult,
    reply_on: &ReplyOn,
) -> EventResult<EvtReply>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_reply(
        vm,
        storage,
        gas_tracker,
        block,
        msg_depth,
        contract,
        msg,
        result,
        reply_on,
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            tracing::info!(contract = contract.to_string(), "Performed reply");
        },
        "Failed to perform reply",
    );

    evt
}

fn _do_reply<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    contract: Addr,
    msg: &Json,
    result: &SubMsgResult,
    reply_on: &ReplyOn,
) -> EventResult<EvtReply>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtReply::base(contract, reply_on.clone());

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
        call_in_2_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            msg_depth,
            0,
            true,
            "reply",
            code_hash,
            &ctx,
            msg,
            result,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
