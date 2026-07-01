#[cfg(feature = "tracing")]
use dango_dyn_event::dyn_event;
use {
    crate::{
        AppError, CHAIN_ID, CONFIG, CONTRACTS, EventResult, GasTracker, TraceOption, Vm,
        call_in_1_out_1_handle_response, catch_and_update_event, catch_event,
    },
    dango_math::{IsZero, MultiplyFraction, NumberConst, Uint128},
    dango_primitives::{
        AuthMode, BankMsg, BlockInfo, Context, EvtWithhold, StdError, Storage, Tx, btree_map, coins,
    },
};

pub fn do_withhold_fee<VM>(
    vm: VM,
    storage: Box<dyn Storage>,
    gas_tracker: GasTracker,
    block: BlockInfo,
    tx: &Tx,
    mode: AuthMode,
    trace_opt: TraceOption,
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let evt = _do_withhold_fee(vm, storage, gas_tracker, block, tx, mode, trace_opt);

    #[cfg(feature = "tracing")]
    evt.debug(
        |_| {
            dyn_event!(
                trace_opt.ok_level.into(),
                sender = tx.sender.to_string(),
                "Withheld fee"
            );
        },
        "Failed to withhold fee",
        trace_opt.error_level.into(),
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
    trace_opt: TraceOption,
) -> EventResult<EvtWithhold>
where
    VM: Vm + Clone + Send + Sync + 'static,
    AppError: From<VM::Error>,
{
    let mut evt = EvtWithhold::base(tx.sender, tx.gas_limit);

    let (cfg, chain_id, bank_code_hash, fee) = catch_event! {
        {
            let cfg = CONFIG.load(&storage)?;
            let chain_id = CHAIN_ID.load(&storage)?;
            let bank_code_hash = CONTRACTS.load(&storage, cfg.bank)?.code_hash;

            // Compute the fee to withhold: `ceil(gas_limit * gas_fee_rate)`.
            // We ceil, rather than floor, to never undercharge.
            //
            // No fee is charged when:
            //
            // 1. simulating a tx. At this time the sender doesn't know how much
            //    gas to request, so the node's query gas limit is used as
            //    `tx.gas_limit`.
            // 2. the sender is exempt. This includes the oracle contract (which
            //    submits Pyth price feeds during `PrepareProposal`) and the
            //    account factory (which onboards new users).
            let fee = if mode == AuthMode::Simulate || cfg.gas_exemptions.contains(&tx.sender) {
                Uint128::ZERO
            } else {
                Uint128::new(tx.gas_limit as u128)
                    .checked_mul_dec_ceil(cfg.gas_fee_rate)
                    .map_err(StdError::from)?
            };

            Ok((cfg, chain_id, bank_code_hash, fee))
        },
        evt
    };

    // If the fee is non-zero, deduct it from the sender and credit the chain's
    // owner. We invoke the bank's `bank_execute` entry point directly -- the
    // same privileged path the state machine uses for regular transfers -- which
    // moves the funds without invoking the recipient's `receive` method.
    //
    // If the sender doesn't have enough funds to cover the fee, this call fails.
    // The failure surfaces as a withhold error, causing the tx to be rejected
    // from the mempool (during `CheckTx`) or aborted (during block execution).
    if fee.is_non_zero() {
        let ctx = Context {
            chain_id,
            block,
            contract: cfg.bank,
            sender: None,
            funds: None,
            mode: None,
        };

        let msg = BankMsg {
            from: tx.sender,
            transfers: btree_map! { cfg.owner => coins! { cfg.gas_token => fee } },
        };

        catch_and_update_event! {
            call_in_1_out_1_handle_response(
                vm,
                storage,
                gas_tracker,
                0,
                0,
                true,
                "bank_execute",
                bank_code_hash,
                &ctx,
                &msg,
                trace_opt,
            ),
            evt => guest_event
        }
    }

    EventResult::Ok(evt)
}
