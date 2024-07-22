use {
    crate::{Config, CONFIG},
    anyhow::ensure,
    grug_types::{
        Coins, Message, MultiplyFraction, MutableCtx, Number, Outcome, Response, StdResult,
        SudoCtx, Tx, Uint128,
    },
};

pub fn initialize(ctx: MutableCtx, config: Config) -> StdResult<Response> {
    CONFIG.save(ctx.storage, &config)?;

    Ok(Response::new())
}

pub fn update_config(ctx: MutableCtx, new_config: &Config) -> anyhow::Result<Response> {
    let info = ctx.querier.query_info()?;

    // Only the chain's owner can update config.
    ensure!(
        ctx.sender == info.config.owner,
        "you don't have the right, O you don't have the right"
    );

    CONFIG.save(ctx.storage, new_config)?;

    Ok(Response::new())
}

pub fn compute_and_transfer_fee(ctx: SudoCtx, tx: Tx, outcome: Outcome) -> StdResult<Response> {
    let cfg = CONFIG.load(ctx.storage)?;
    let info = ctx.querier.query_info()?;

    // Compute the fee amount.
    // Note that we ceil the amount instead of flooring.
    let fee_amount = Uint128::from(outcome.gas_used).checked_mul_dec_ceil(cfg.fee_rate)?;

    // Call the bank contract's `force_transfer` method to move the fee tokens
    // from the transaction's sender to the owner.
    //
    // If the transaction's sender is the owner, or if fee amount is zero, then
    // we don't need to do anything.
    let maybe_msg = if tx.sender == info.config.owner || fee_amount.is_zero() {
        None
    } else {
        Some(Message::execute(
            info.config.bank,
            &grug_bank::ExecuteMsg::ForceTransfer {
                from: tx.sender,
                to: info.config.owner,
                denom: cfg.fee_denom,
                amount: fee_amount,
            },
            Coins::new(),
        )?)
    };

    Ok(Response::new().maybe_add_message(maybe_msg))
}
