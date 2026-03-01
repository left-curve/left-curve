use {
    crate::{USER_STATES, execute::ORACLE},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{Quantity, perps::settlement_currency},
    grug::{IsZero, MutableCtx, Response},
};

/// Deposit settlement currency into the trader's margin account.
/// The deposited tokens are converted to USD at the current oracle price and
/// credited to `user_state.margin`. Tokens stay in the perps contract's bank
/// balance.
///
/// Mutates: `USER_STATES` (margin increased).
///
/// Returns: empty `Response` (no outgoing messages).
pub fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // Extract settlement currency from funds.
    let mut funds = ctx.funds;
    let deposit_amount = funds.take(settlement_currency::DENOM.clone()).amount;

    ensure!(funds.is_empty(), "unexpected deposit: {funds:?}");
    ensure!(deposit_amount.is_non_zero(), "nothing to deposit");

    // Convert to USD value.
    let deposit_value = Quantity::from_base(deposit_amount, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Credit to user's margin.
    user_state.margin.checked_add_assign(deposit_value)?;

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    Ok(Response::new())
}
