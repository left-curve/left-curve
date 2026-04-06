use {
    crate::state::{STATE, USER_STATES},
    anyhow::ensure,
    dango_types::UsdValue,
    grug::{MutableCtx, Response},
};

/// Replenish the insurance fund from the sender's margin.
pub fn replenish_insurance_fund(ctx: MutableCtx, amount: UsdValue) -> anyhow::Result<Response> {
    ensure!(amount.is_non_zero(), "nothing to replenish");

    // Deduct the user's margin.
    {
        let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

        ensure!(
            user_state.margin >= amount,
            "insufficient margin: have {}, want {}",
            user_state.margin,
            amount,
        );

        user_state.margin.checked_sub_assign(amount)?;

        if user_state.is_empty() {
            USER_STATES.remove(ctx.storage, ctx.sender)?;
        } else {
            USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
        }
    }

    // Increment the insurance fund's balance.
    {
        let mut state = STATE.load(ctx.storage)?;

        state.insurance_fund.checked_add_assign(amount)?;

        STATE.save(ctx.storage, &state)?;
    }

    Ok(Response::new())
}
