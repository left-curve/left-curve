use {
    crate::{
        call_in_1_out_1_handle_response, catch_and_update_event, catch_event, AppError,
        EventResult, GasTracker, Vm, _do_transfer, CHAIN_ID, CONTRACTS,
    },
    grug_types::{Addr, BlockInfo, Context, EvtExecute, MsgExecute, MsgTransfer, Storage},
};

pub fn do_execute<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgExecute,
) -> EventResult<EvtExecute>
where
    VM: Vm + Clone,
    AppError: From<VM::Error>,
{
    let evt = _do_execute(
        vm,
        storage,
        gas_tracker,
        block,
        msg_depth,
        sender,
        msg.clone(),
    );

    #[cfg(feature = "tracing")]
    evt.debug(
        |evt| {
            tracing::info!(contract = evt.contract.to_string(), "Executed contract");
        },
        "Failed to execute contract",
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
) -> EventResult<EvtExecute>
where
    VM: Vm + Clone,
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
                MsgTransfer {
                    to: msg.contract,
                    coins: msg.funds.clone(),
                },
                false,
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
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
