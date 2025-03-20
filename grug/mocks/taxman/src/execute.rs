use {
    crate::{CONFIG, Config, WITHHELD_FEE},
    grug_math::{IsZero, MultiplyFraction, Number, NumberConst, Uint128},
    grug_types::{
        AuthCtx, AuthMode, Coins, Message, QuerierExt, Response, StdResult, Storage, Tx, TxOutcome,
    },
};

pub fn initialize_config(storage: &mut dyn Storage, cfg: &Config) -> StdResult<Response> {
    CONFIG.save(storage, cfg)?;

    Ok(Response::new())
}

pub fn withhold_fee(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    let fee_cfg = CONFIG.load(ctx.storage)?;

    // Compute the maximum amount of fee this transaction may incur.
    // Note that we ceil the amount here, instead of flooring.
    // In simulation mode, don't do anything.
    let withhold_amount = if ctx.mode == AuthMode::Simulate {
        Uint128::ZERO
    } else {
        Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?
    };

    // If the fee amount is non-zero, we force transfer the max fee amount from
    // the sender to here (the taxman). If zero, nothing to do.
    //
    // If the sender doesn't have enough coin balance to cover the max fee, this
    // submessage would error, causing the tx to be aborted.
    //
    // Since `withhold_fee` is called during `CheckTx`, this prevents an
    // attacker from spamming txs into the mempool when he doesn't have enough
    // coins.
    let withhold_msg = if withhold_amount.is_non_zero() {
        let bank = ctx.querier.query_bank()?;
        Some(Message::execute(
            bank,
            &grug_mock_bank::ExecuteMsg::ForceTransfer {
                from: tx.sender,
                to: ctx.contract,
                denom: fee_cfg.fee_denom.clone(),
                amount: withhold_amount,
            },
            Coins::new(),
        )?)
    } else {
        None
    };

    // Save the withheld fee in storage, which we will use in `finalize_fee`.
    WITHHELD_FEE.save(ctx.storage, &(fee_cfg, withhold_amount))?;

    Ok(Response::new().may_add_message(withhold_msg))
}

pub fn finalize_fee(ctx: AuthCtx, tx: Tx, outcome: TxOutcome) -> anyhow::Result<Response> {
    let (fee_cfg, withheld_amount) = WITHHELD_FEE.take(ctx.storage)?;

    // Compute the amount of fee that will actually be charged, based on actual
    // gas consumption.
    //
    // Same as withholding, we ceil here instead of flooring.
    let charge_amount = if ctx.mode == AuthMode::Simulate {
        Uint128::ZERO
    } else {
        Uint128::new(outcome.gas_used as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?
    };

    // If we have withheld more funds than the actual charge amount, we need to
    // refund the difference.
    let refund_amount = withheld_amount.saturating_sub(charge_amount);

    let refund_msg = if refund_amount.is_non_zero() {
        let bank = ctx.querier.query_bank()?;
        Some(Message::execute(
            bank,
            &grug_mock_bank::ExecuteMsg::ForceTransfer {
                from: ctx.contract,
                to: tx.sender,
                denom: fee_cfg.fee_denom,
                amount: refund_amount,
            },
            Coins::new(),
        )?)
    } else {
        None
    };

    Ok(Response::new().may_add_message(refund_msg))
}
