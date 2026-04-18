use {
    crate::USER_STATES,
    anyhow::ensure,
    dango_types::{
        Quantity,
        perps::{Deposited, SETTLEMENT_CURRENCY_PRICE, settlement_currency},
    },
    grug::{Addr, IsZero, MutableCtx, Response},
};

/// Deposit settlement currency into the trader's margin account.
///
/// The deposited tokens are converted to USD at a fixed 1:1 rate and credited
/// to `user_state.margin`. Tokens stay in the perps contract's bank balance.
pub fn deposit(mut ctx: MutableCtx, to: Option<Addr>) -> anyhow::Result<Response> {
    // ----------------------- 1. Extract deposit amount -----------------------

    let deposit_amount = ctx.funds.take(settlement_currency::DENOM.clone()).amount;

    ensure!(deposit_amount.is_non_zero(), "nothing to deposit");

    ensure!(ctx.funds.is_empty(), "unexpected deposit: {}", ctx.funds);

    // -------------------- 2. Convert deposit to USD value --------------------

    let deposit_value = Quantity::from_base(deposit_amount, settlement_currency::DECIMAL)?
        .checked_mul(SETTLEMENT_CURRENCY_PRICE)?;

    // ------------------------- 3. Update user state --------------------------

    let to = to.unwrap_or(ctx.sender);

    let mut user_state = USER_STATES.may_load(ctx.storage, to)?.unwrap_or_default();

    user_state.margin.checked_add_assign(deposit_value)?;

    USER_STATES.save(ctx.storage, to, &user_state)?;

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            user = %to,
            %deposit_value,
            "Margin deposited"
        );
    }

    #[cfg(feature = "metrics")]
    {
        metrics::histogram!(crate::metrics::LABEL_DEPOSIT_AMOUNT).record(deposit_value.to_f64());
    }

    Ok(Response::new().add_event(Deposited {
        user: to,
        amount: deposit_value,
    })?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::USER_STATES,
        dango_types::UsdValue,
        grug::{Addr, Coins, MockContext, ResultExt, Uint128},
    };

    const SENDER: Addr = Addr::mock(1);
    const RECIPIENT: Addr = Addr::mock(2);

    /// 1 USDC = 1_000_000 base units (6 decimals).
    fn usdc_coins(amount: u128) -> Coins {
        Coins::one(
            settlement_currency::DENOM.clone(),
            Uint128::new(amount * 1_000_000),
        )
        .unwrap()
    }

    #[test]
    fn deposit_to_another_account() {
        let mut ctx = MockContext::new()
            .with_sender(SENDER)
            .with_funds(usdc_coins(1_000));

        deposit(ctx.as_mutable(), Some(RECIPIENT)).should_succeed();

        // Recipient should have the deposited margin.
        let recipient_state = USER_STATES.load(&ctx.storage, RECIPIENT).unwrap();
        assert_eq!(recipient_state.margin, UsdValue::new_int(1_000));

        // Sender should have no state — funds went to the recipient.
        assert!(
            USER_STATES
                .may_load(&ctx.storage, SENDER)
                .unwrap()
                .is_none()
        );
    }
}
