use {
    crate::{
        MAX_ORACLE_STALENESS, core::compute_available_margin, oracle, querier::NoCachePerpQuerier,
        state::USER_STATES,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        UsdValue,
        perps::{SETTLEMENT_CURRENCY_PRICE, Withdrew, settlement_currency},
    },
    grug::{IsZero, Message, MutableCtx, Response, coins},
};

/// Withdraw margin from the trader's margin account.
/// The requested USD amount is validated against the user's available margin,
/// deducted from `user_state.margin`, converted to settlement currency at a
/// fixed 1:1 rate (floor-rounded), and transferred to the user.
///
/// Mutates: `USER_STATES` (margin decreased, possibly removed if empty).
///
/// Returns: `Response` with a transfer message.
pub fn withdraw(ctx: MutableCtx, amount: UsdValue) -> anyhow::Result<Response> {
    ensure!(
        amount.is_positive(),
        "can only withdraw positive amount of margin"
    );

    // ---------------------- 1. Compute available margin ----------------------

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let available = compute_available_margin(&mut oracle_querier, &perp_querier, &user_state)?;

    ensure!(
        amount <= available,
        "withdrawal amount ({amount}) exceeds available margin ({available})"
    );

    // ----------------------- 2. Compute refund amount ------------------------

    // Convert USD to settlement currency base units (floor-rounded).
    // Use a fixed 1:1 rate to guarantee solvency — total USDC held always
    // equals total USD margin across all accounts.
    let refund = amount
        .checked_div(SETTLEMENT_CURRENCY_PRICE)?
        .into_base_floor(settlement_currency::DECIMAL)?;

    ensure!(
        refund.is_non_zero(),
        "refund amount rounds down to zero tokens"
    );

    // ------------------- 3. Update and persist user state --------------------

    user_state.margin.checked_sub_assign(amount)?;

    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.sender)?;
    } else {
        USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            user = %ctx.sender,
            %amount,
            "Margin withdrawn"
        );
    }

    #[cfg(feature = "metrics")]
    {
        metrics::histogram!(crate::metrics::LABEL_WITHDRAWAL_AMOUNT).record(amount.to_f64());
    }

    Ok(Response::new()
        .add_message(Message::transfer(
            ctx.sender,
            coins! { settlement_currency::DENOM.clone() => refund },
        )?)
        .add_event(Withdrew {
            user: ctx.sender,
            amount,
        })?)
}
