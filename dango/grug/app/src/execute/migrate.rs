#[cfg(feature = "tracing")]
use dyn_event::dyn_event;
use {
    crate::{
        AppError, CHAIN_ID, CODES, CONTRACTS, EventResult, GasTracker, TraceOption, Vm,
        call_in_1_out_1_handle_response, catch_and_update_event, catch_event,
    },
    grug_types::{
        Addr, BlockInfo, CodeStatus, Context, EvtMigrate, MsgMigrate, StdResult, Storage,
    },
};

pub fn do_migrate<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgMigrate,
    trace_opt: TraceOption,
) -> EventResult<EvtMigrate>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_migrate(
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
                trace_opt.ok_level.into(),
                contract = evt.contract.to_string(),
                "Migrated contract"
            );
        },
        "Failed to migrate contract",
        trace_opt.error_level.into(),
    );

    evt
}

fn _do_migrate<VM>(
    vm: VM,
    mut storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    msg_depth: usize,
    sender: Addr,
    msg: MsgMigrate,
    trace_opt: TraceOption,
) -> EventResult<EvtMigrate>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtMigrate::base(sender, msg.contract, msg.msg.clone(), msg.new_code_hash);

    let (old_code_hash, chain_id) = catch_event! {
        {
            // Update the contract info.
            let mut old_code_hash = None;

            CONTRACTS.update(&mut storage, msg.contract, |mut info| {
                old_code_hash = Some(info.code_hash);

                // Ensure the sender is the admin of the contract.
                if Some(sender) != info.admin {
                    return Err(AppError::unauthorized());
                }

                info.code_hash = msg.new_code_hash;

                Ok(info)
            })?;

            let old_code_hash = old_code_hash.unwrap();

            // Reduce usage count of the old code.
            CODES.update(&mut storage, old_code_hash, |mut code| -> StdResult<_> {
                match &mut code.status {
                    CodeStatus::InUse { usage } => {
                        if *usage == 1 {
                            code.status = CodeStatus::Orphaned {
                                since: block.timestamp,
                            };
                        } else {
                            *usage -= 1;
                        }
                    },
                    _ => unreachable!(),
                }

                Ok(code)
            })?;

            // Increase usage count of the new code.
            CODES.update(&mut storage, msg.new_code_hash, |mut code| -> StdResult<_> {
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

            let chain_id = CHAIN_ID.load(&storage)?;

            Ok((old_code_hash, chain_id))
        },
        evt
    };

    let ctx = Context {
        chain_id,
        block,
        contract: msg.contract,
        sender: None,
        funds: None,
        mode: None,
    };

    evt.old_code_hash = Some(old_code_hash);

    catch_and_update_event! {
        call_in_1_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            msg_depth,
            0,
            true,
            "migrate",
            msg.new_code_hash,
            &ctx,
            &msg.msg,
            trace_opt,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
