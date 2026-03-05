use {
    crate::{USER_STATES, execute::oracle},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity,
        perps::{Deposited, settlement_currency},
    },
    grug::{IsZero, MutableCtx, Response},
};

/// Deposit settlement currency into the trader's margin account.
///
/// The deposited tokens are converted to USD at the current oracle price and
/// credited to `user_state.margin`. Tokens stay in the perps contract's bank
/// balance.
pub fn deposit(mut ctx: MutableCtx) -> anyhow::Result<Response> {
    // ----------------------- 1. Extract deposit amount -----------------------

    let deposit_amount = ctx.funds.take(settlement_currency::DENOM.clone()).amount;

    ensure!(deposit_amount.is_non_zero(), "nothing to deposit");

    ensure!(ctx.funds.is_empty(), "unexpected deposit: {}", ctx.funds);

    // -------------------- 2. Convert deposit to USD value --------------------

    let settlement_currency_price = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier)
        .query_price_for_perps(&settlement_currency::DENOM)?;

    let deposit_value = Quantity::from_base(deposit_amount, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // ------------------------- 3. Update user state --------------------------

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    user_state.margin.checked_add_assign(deposit_value)?;

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    Ok(Response::new().add_event(Deposited {
        user: ctx.sender,
        amount: deposit_value,
    })?)
}
