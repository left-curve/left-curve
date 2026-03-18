use {
    crate::USER_STATES,
    anyhow::ensure,
    dango_types::{
        Quantity,
        perps::{Deposited, SETTLEMENT_CURRENCY_PRICE, settlement_currency},
    },
    grug::{IsZero, MutableCtx, Response},
};

/// Deposit settlement currency into the trader's margin account.
///
/// The deposited tokens are converted to USD at a fixed 1:1 rate and credited
/// to `user_state.margin`. Tokens stay in the perps contract's bank balance.
pub fn deposit(mut ctx: MutableCtx) -> anyhow::Result<Response> {
    // ----------------------- 1. Extract deposit amount -----------------------

    let deposit_amount = ctx.funds.take(settlement_currency::DENOM.clone()).amount;

    ensure!(deposit_amount.is_non_zero(), "nothing to deposit");

    ensure!(ctx.funds.is_empty(), "unexpected deposit: {}", ctx.funds);

    // -------------------- 2. Convert deposit to USD value --------------------

    let deposit_value = Quantity::from_base(deposit_amount, settlement_currency::DECIMAL)?
        .checked_mul(SETTLEMENT_CURRENCY_PRICE)?;

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
