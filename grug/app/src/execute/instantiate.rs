#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        _do_transfer, AppError, CHAIN_ID, CODES, CONFIG, CONTRACTS, EventResult, GasTracker,
        TraceOption, Vm, call_in_1_out_1_handle_response, catch_and_update_event, catch_event,
        has_permission,
    },
    grug_types::{
        Addr, BlockInfo, CodeStatus, Context, ContractInfo, EvtInstantiate, MsgInstantiate,
        StdResult, Storage, btree_map,
    },
};

pub fn do_instantiate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgInstantiate,
    trace_opt: TraceOption,
) -> EventResult<EvtInstantiate>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_instantiate(
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
                address = evt.contract.to_string(),
                "Instantiated contract"
            );
        },
        "Failed to instantiate contract",
        trace_opt.error_level,
    );

    evt
}

pub fn _do_instantiate<VM>(
    vm: VM,
    mut storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgInstantiate,
    trace_opt: TraceOption,
) -> EventResult<EvtInstantiate>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    // Compute the contract address, and make sure there isn't already a
    // contract of the same address.
    let address = Addr::derive(sender, msg.code_hash, &msg.salt);

    let mut evt = EvtInstantiate::base(sender, msg.code_hash, address, msg.msg.clone());

    let chain_id = catch_event! {
        {
            let cfg = CONFIG.load(&storage)?;

            // Make sure the user has the permission to instantiate contracts
            if !has_permission(&cfg.permissions.instantiate, cfg.owner, sender) {
                return Err(AppError::Unauthorized);
            }

            // Save the contract info
            CONTRACTS.may_update(&mut storage, address, |maybe_contract| {
                if maybe_contract.is_some() {
                    return Err(AppError::AccountExists { address });
                }

                Ok(ContractInfo {
                    code_hash: msg.code_hash,
                    label: msg.label,
                    admin: msg.admin,
                })
            })?;

            // Increment the code's usage.
            CODES.update(&mut storage, msg.code_hash, |mut code| -> StdResult<_> {
                match &mut code.status {
                    CodeStatus::Orphaned { .. } => {
                        code.status = CodeStatus::InUse { usage: 1 };
                    },
                    CodeStatus::InUse { usage } => {
                        *usage += 1;
                    },
                }

                Ok(code)
            })?;

            Ok(CHAIN_ID.load(&storage)?)
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
                btree_map! { address => msg.funds.clone() },
                false,
                trace_opt,
            ),
            evt => transfer_event
        }
    }

    // Call the contract's `instantiate` entry point
    let ctx = Context {
        chain_id,
        block,
        contract: address,
        sender: Some(sender),
        funds: Some(msg.funds),
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
            "instantiate",
            msg.code_hash,
            &ctx,
            &msg.msg,
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
