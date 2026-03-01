use {
    crate::{NoCachePerpQuerier, USER_STATES, core::compute_available_margin, execute::ORACLE},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{UsdValue, perps::settlement_currency},
    grug::{IsZero, Message, MutableCtx, Response, coins},
};

/// Withdraw margin from the trader's margin account.
/// The requested USD amount is validated against the user's available margin,
/// deducted from `user_state.margin`, converted to settlement currency at the
/// current oracle price (floor-rounded), and transferred to the user.
///
/// Mutates: `USER_STATES` (margin decreased, possibly removed if empty).
///
/// Returns: `Response` with a transfer message.
pub fn withdraw(ctx: MutableCtx, margin: UsdValue) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    ensure!(margin.is_positive(), "nothing to withdraw");

    // Compute available margin (equity - initial margin - reserved margin).
    let available = compute_available_margin(
        user_state.margin,
        &user_state,
        &perp_querier,
        &mut oracle_querier,
        user_state.reserved_margin,
    )?;

    ensure!(
        margin <= available,
        "withdrawal of {margin} exceeds available margin of {available}"
    );

    // Deduct from margin.
    user_state.margin.checked_sub_assign(margin)?;

    // Convert USD to settlement currency base units (floor-rounded).
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    let quantity = margin.checked_div(settlement_currency_price)?;
    let amount = quantity.into_base_floor(settlement_currency::DECIMAL)?;

    ensure!(
        amount.is_non_zero(),
        "withdrawal amount rounds to zero tokens"
    );

    // Persist updated user state (or remove if empty).
    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.sender);
    } else {
        USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    }

    // Transfer settlement currency to the user.
    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        coins! { settlement_currency::DENOM.clone() => amount },
    )?))
}
