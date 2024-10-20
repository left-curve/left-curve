use {
    crate::CONFIG,
    anyhow::ensure,
    dango_types::{
        bank,
        config::ACCOUNT_FACTORY_KEY,
        taxman::{Config, ExecuteMsg, InstantiateMsg},
    },
    grug::{
        Addr, AuthCtx, AuthMode, Coins, IsZero, Message, MultiplyFraction, MutableCtx, Number,
        Response, StdResult, Tx, TxOutcome, Uint128,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    CONFIG.save(ctx.storage, &msg.config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { new_cfg } => update_config(ctx, new_cfg),
        ExecuteMsg::Pay { payer } => pay(ctx, payer),
    }
}

fn update_config(ctx: MutableCtx, new_cfg: Config) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    // Only the chain's owner can update fee config.
    ensure!(
        ctx.sender == cfg.owner,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.save(ctx.storage, &new_cfg)?;

    Ok(Response::new())
}

fn pay(_ctx: MutableCtx, _payer: Addr) -> anyhow::Result<Response> {
    // For now, nothing to do.
    // In the future, we will implement affiliate fees.
    Ok(Response::new())
}

// TODO: exempt the account factory from paying fee.
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn withhold_fee(ctx: AuthCtx, tx: Tx) -> StdResult<Response> {
    let fee_cfg = CONFIG.load(ctx.storage)?;
    let account_factory: Addr = ctx.querier.query_app_config("account_factory")?;

    // Two situations where gas handling is skipped:
    // 1. During simulation, no need to do anything.
    // 2. The account factory contract is exempt from gas fees.
    if ctx.mode == AuthMode::Simulate || tx.sender == account_factory {
        return Ok(Response::new());
    }

    // Compute the maximum amount of fee this transaction may incur.
    //
    // Note that we ceil this amount, instead of flooring.
    let withhold_amount =
        Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?;

    // If the withhold amount is non-zero, we force transfer this amount from
    // the sender to taxman.
    //
    // If the sender doesn't have enough fund to cover the maximum amount of fee
    // the tx may incur, this submessage fails, causing the tx to be rejected
    // from entering the mempool.
    let withhold_msg = if withhold_amount.is_non_zero() {
        // TODO: for production, we can hardcode the bank contract address
        // instead of having to make the query.
        let cfg = ctx.querier.query_config()?;

        Some(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::ForceTransfer {
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

    Ok(Response::new().may_add_message(withhold_msg))
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn finalize_fee(ctx: AuthCtx, tx: Tx, outcome: TxOutcome) -> StdResult<Response> {
    let fee_cfg = CONFIG.load(ctx.storage)?;
    let account_factory: Addr = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;

    // Again, during simulation, or any tx sent by the account factory, is
    // exempt from gas fees.
    if ctx.mode == AuthMode::Simulate || tx.sender == account_factory {
        return Ok(Response::new());
    }

    // Compute how much fee was withheld earlier during `withhold_fee`.
    //
    // FIXME: this doesn't work if the fee rate was changed during this tx!!!
    // Instead of recomputing, we should save this in the storage.
    let withheld_amount =
        Uint128::new(tx.gas_limit as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?;

    // Compute how much fee to charge the sender, based on the actual amount of
    // gas consumed.
    let charge_amount =
        Uint128::new(outcome.gas_used as u128).checked_mul_dec_ceil(fee_cfg.fee_rate)?;

    // If we have withheld more funds than the actual charge amount, we need to
    // refund the difference.
    let refund_amount = withheld_amount.saturating_sub(charge_amount);

    let refund_msg = if refund_amount.is_non_zero() {
        Some(Message::transfer(
            tx.sender,
            Coins::one(fee_cfg.fee_denom, refund_amount)?,
        )?)
    } else {
        None
    };

    Ok(Response::new().may_add_message(refund_msg))
}
