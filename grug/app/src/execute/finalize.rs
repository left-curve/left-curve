use {
    crate::{
        call_in_2_out_1_handle_response, catch_and_update_event, catch_event, AppError,
        EventResult, GasTracker, Vm, CHAIN_ID, CONFIG, CONTRACTS,
    },
    grug_types::{AuthMode, BlockInfo, Context, EvtFinalize, Storage, Tx, TxOutcome},
};

pub fn do_finalize_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    outcome: &TxOutcome,
    mode: AuthMode,
) -> EventResult<EvtFinalize>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_finalize_fee(vm, storage, gas_tracker, block, tx, outcome, mode);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            tracing::info!(sender = tx.sender.to_string(), "Finalized fee");
        },
        "Failed to finalize fee",
        // `finalize_fee` is supposed to always succeed, so if it doesn't,
        // we print a tracing log at ERROR level to highlight the seriousness.
        // #[cfg(feature = "tracing")]
        // tracing::error!(err = err.to_string(), "Failed to finalize fee")
    );

    evt
}

pub fn _do_finalize_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    outcome: &TxOutcome,
    mode: AuthMode,
) -> EventResult<EvtFinalize>
where
    VM: Vm + Clone + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtFinalize::base(tx.sender, tx.gas_limit, outcome.gas_used);

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
        call_in_2_out_1_handle_response(
            vm,
            storage,
            gas_tracker,
            0,
            0,
            true,
            "finalize_fee",
            taxman.code_hash,
            &ctx,
            tx,
            outcome,
        ),
        evt => guest_event
    }

    EventResult::Ok(evt)
}
