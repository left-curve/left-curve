use {
    crate::{
        AppError, CHAIN_ID, CONFIG, CONTRACTS, EventResult, GasTracker, Vm,
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
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_withhold_fee(vm, storage, gas_tracker, block, tx, mode);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            tracing::info!(sender = tx.sender.to_string(), "Withheld fee");
        },
        "Failed to withhold fee",
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
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone + 'static,
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
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
