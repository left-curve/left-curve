use {
    crate::{Config, CONFIG},
    anyhow::{bail, ensure},
    grug_types::{
        Coins, Message, MultiplyFraction, MutableCtx, NonZero, Number, Outcome, Response,
        StdResult, Storage, SudoCtx, Tx, Uint128,
    },
};

pub fn initialize_config(storage: &mut dyn Storage, cfg: &Config) -> StdResult<Response> {
    CONFIG.save(storage, &cfg)?;

    Ok(Response::new())
}

pub fn update_config(ctx: MutableCtx, new_cfg: &Config) -> anyhow::Result<Response> {
    let info = ctx.querier.query_info()?;

    // Only the chain's owner can update fee config.
    ensure!(
        Some(ctx.sender) == info.config.owner,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.save(ctx.storage, new_cfg)?;

    Ok(Response::new())
}

pub fn withhold_fee(ctx: SudoCtx, tx: Tx) -> StdResult<Response> {
    let cfg = CONFIG.load(ctx.storage)?;
    let info = ctx.querier.query_info()?;

    // Compute the maximum amount of fee this transaction may incur.
    //
    // Note that we ceil the amount here, instead of flooring.
    let withhold_amount = Uint128::from(tx.gas_limit).checked_mul_dec_ceil(cfg.fee_rate)?;

    // If the fee amount is non-zero, we force transfer the max fee amount from
    // the sender to here (the taxman). If zero, nothing to do.
    //
    // If the sender doesn't have enough coin balance to cover the max fee, this
    // submessage would error, causing the tx to be aborted.
    //
    // Since `withhold_fee` is called during `CheckTx`, this prevents an
    // attacker from spamming txs into the mempool when he doesn't have enough
    // coins.
    let withhold_msg = if !withhold_amount.is_zero() {
        Some(Message::execute(
            info.config.bank,
            &grug_bank::ExecuteMsg::ForceTransfer {
                from: tx.sender,
                to: ctx.contract,
                denom: cfg.fee_denom,
                amount: withhold_amount,
            },
            Coins::new(),
        )?)
    } else {
        None
    };

    Ok(Response::new().may_add_message(withhold_msg))
}

pub fn finalize_fee(ctx: SudoCtx, tx: Tx, outcome: Outcome) -> anyhow::Result<Response> {
    let cfg = CONFIG.load(ctx.storage)?;
    let info = ctx.querier.query_info()?;

    // Compute the amount of fee that was withheld during `withheld fee`.
    let withheld_amount = Uint128::from(tx.gas_limit).checked_mul_dec_ceil(cfg.fee_rate)?;

    // Compute the amount of fee that will actually be charged, based on actual
    // gas consumption.
    //
    // Note that we floor here, instead of ceiling.
    let charge_amount = Uint128::from(outcome.gas_used).checked_mul_dec_floor(cfg.fee_rate)?;

    // The difference between the two amounts is to be refunded to the user.
    let refund_amount = withheld_amount.saturating_sub(charge_amount);

    // If the charge amount if non-zero, we send the fee to the chain's owner.
    //
    // This is just a demo. In practice, it probably makes more sense to have a
    // fee distributor contract that distribute to stakers so something like that.
    let charge_msg = if !charge_amount.is_zero() {
        let Some(owner) = info.config.owner else {
            bail!("chain owner is not set!");
        };

        Some(Message::Transfer {
            to: owner,
            coins: Coins::one(cfg.fee_denom.clone(), NonZero::new(charge_amount)),
        })
    } else {
        None
    };

    let refund_msg = if !refund_amount.is_zero() {
        Some(Message::Transfer {
            to: tx.sender,
            coins: Coins::one(cfg.fee_denom, NonZero::new(refund_amount)),
        })
    } else {
        None
    };

    Ok(Response::new()
        .may_add_message(charge_msg)
        .may_add_message(refund_msg)
        .add_attribute("charge_amount", charge_amount)
        .add_attribute("refund_amount", refund_amount))
}
