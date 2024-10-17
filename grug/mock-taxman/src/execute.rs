use {
    crate::{Config, CONFIG},
    anyhow::ensure,
    grug_math::{IsZero, MultiplyFraction, Number, Uint128},
    grug_types::{
        AuthCtx, AuthMode, Coins, Message, MutableCtx, Response, StdResult, Storage, Tx, TxOutcome,
    },
};

pub fn initialize_config(storage: &mut dyn Storage, cfg: &Config) -> StdResult<Response> {
    CONFIG.save(storage, cfg)?;

    Ok(Response::new())
}

pub fn update_config(ctx: MutableCtx, new_cfg: &Config) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    // Only the chain's owner can update fee config.
    ensure!(
        ctx.sender == cfg.owner,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.save(ctx.storage, new_cfg)?;

    Ok(Response::new())
}

pub fn withhold_fee(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    let cfg = ctx.querier.query_config()?;
    let fee_cfg = CONFIG.load(ctx.storage)?;

    // In simulation mode, don't do anything.
    if ctx.mode == AuthMode::Simulate {
        return Ok(Response::new());
    }

    // Compute the maximum amount of fee this transaction may incur.
    //
    // Note that we ceil the amount here, instead of flooring.
    let withhold_amount =
        Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?;

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
        Some(Message::execute(
            cfg.bank,
            &grug_mock_bank::ExecuteMsg::ForceTransfer {
                from: tx.sender,
                to: ctx.contract,
                denom: fee_cfg.fee_denom,
                amount: withhold_amount,
            },
            Coins::new(),
        )?)
    } else {
        None
    };

    Ok(Response::new()
        .may_add_message(withhold_msg)
        .add_attribute("gas_limit", tx.gas_limit)
        .add_attribute("withhold_amount", withhold_amount))
}

pub fn finalize_fee(ctx: AuthCtx, tx: Tx, outcome: TxOutcome) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;
    let fee_cfg = CONFIG.load(ctx.storage)?;

    // In simulation mode, don't do anything.
    if ctx.mode == AuthMode::Simulate {
        return Ok(Response::new());
    }

    // Compute the amount of fee that was withheld during `withheld fee`.
    let withheld_amount =
        Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?;

    // Compute the amount of fee that will actually be charged, based on actual
    // gas consumption.
    //
    // Same as withholding, we ceil here instead of flooring.
    let charge_amount =
        Uint128::new(outcome.gas_used as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?;

    // The difference between the two amounts is to be refunded to the user.
    let refund_amount = withheld_amount.saturating_sub(charge_amount);

    // If the charge amount if non-zero, we send the fee to the chain's owner.
    //
    // This is just a demo. In practice, it probably makes more sense to have a
    // fee distributor contract that distribute to stakers so something like that.
    let charge_msg = if charge_amount.is_non_zero() {
        Some(Message::transfer(
            cfg.owner,
            Coins::one(fee_cfg.fee_denom.clone(), charge_amount)?,
        )?)
    } else {
        None
    };

    let refund_msg = if refund_amount.is_non_zero() {
        Some(Message::transfer(
            tx.sender,
            Coins::one(fee_cfg.fee_denom, refund_amount)?,
        )?)
    } else {
        None
    };

    Ok(Response::new()
        .may_add_message(charge_msg)
        .may_add_message(refund_msg)
        .add_attribute("gas_limit", tx.gas_limit)
        .add_attribute("gas_used", outcome.gas_used)
        .add_attribute("withheld_amount", withheld_amount)
        .add_attribute("charge_amount", charge_amount)
        .add_attribute("refund_amount", refund_amount))
}
