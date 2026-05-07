use {
    crate::state::{STATE, USER_STATES},
    anyhow::ensure,
    dango_types::UsdValue,
    grug::{MutableCtx, QuerierExt, Response},
};

/// Move the entire protocol treasury balance into the chain owner's
/// `UserState.margin`. The owner can then convert it to USDC via the
/// regular [`TraderMsg::Withdraw`](dango_types::perps::TraderMsg::Withdraw)
/// flow.
///
/// Treasury is internal accounting (`UsdValue`); the actual USDC backing it
/// already sits in the contract's bank balance. No bank transfer is needed
/// here — only a state-level credit/debit.
pub fn withdraw_from_treasury(ctx: MutableCtx) -> anyhow::Result<Response> {
    let owner = ctx.querier.query_owner()?;

    ensure!(
        ctx.sender == owner,
        "you don't have the right, O you don't have the right"
    );

    let mut state = STATE.load(ctx.storage)?;
    let amount = state.treasury;

    if amount.is_zero() {
        return Ok(Response::new());
    }

    state.treasury = UsdValue::ZERO;
    STATE.save(ctx.storage, &state)?;

    let mut user_state = USER_STATES
        .may_load(ctx.storage, owner)?
        .unwrap_or_default();
    user_state.margin.checked_add_assign(amount)?;
    USER_STATES.save(ctx.storage, owner, &user_state)?;

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::perps::{State, UserState},
        grug::{
            Addr, Coins, Config, Duration, MockContext, MockQuerier, Permission, Permissions,
            ResultExt,
        },
        std::collections::BTreeMap,
    };

    const OWNER: Addr = Addr::mock(0);
    const NON_OWNER: Addr = Addr::mock(1);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(2),
            taxman: Addr::mock(3),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Nobody,
                instantiate: Permission::Nobody,
            },
            max_orphan_age: Duration::from_seconds(0),
        }
    }

    #[test]
    fn non_owner_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(NON_OWNER)
            .with_funds(Coins::default());

        STATE
            .save(&mut ctx.storage, &State {
                treasury: UsdValue::new_int(100),
                ..Default::default()
            })
            .unwrap();

        withdraw_from_treasury(ctx.as_mutable()).should_fail_with_error("you don't have the right");
    }

    #[test]
    fn zero_treasury_noop() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        STATE.save(&mut ctx.storage, &State::default()).unwrap();

        withdraw_from_treasury(ctx.as_mutable()).should_succeed();

        // Owner's UserState should not have been touched.
        assert!(USER_STATES.may_load(&ctx.storage, OWNER).unwrap().is_none());
    }

    #[test]
    fn owner_withdraws_treasury_succeeds() {
        let amount = UsdValue::new_int(1_000);
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        STATE
            .save(&mut ctx.storage, &State {
                treasury: amount,
                ..Default::default()
            })
            .unwrap();

        withdraw_from_treasury(ctx.as_mutable()).should_succeed();

        // Treasury cleared.
        let state = STATE.load(&ctx.storage).unwrap();
        assert_eq!(state.treasury, UsdValue::ZERO);

        // Owner's UserState credited.
        let user_state = USER_STATES.load(&ctx.storage, OWNER).unwrap();
        assert_eq!(user_state.margin, amount);
    }

    #[test]
    fn owner_withdraws_adds_to_existing_margin() {
        let prior_margin = UsdValue::new_int(200);
        let treasury_amount = UsdValue::new_int(500);
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        STATE
            .save(&mut ctx.storage, &State {
                treasury: treasury_amount,
                ..Default::default()
            })
            .unwrap();
        USER_STATES
            .save(&mut ctx.storage, OWNER, &UserState {
                margin: prior_margin,
                ..Default::default()
            })
            .unwrap();

        withdraw_from_treasury(ctx.as_mutable()).should_succeed();

        let state = STATE.load(&ctx.storage).unwrap();
        assert_eq!(state.treasury, UsdValue::ZERO);

        let user_state = USER_STATES.load(&ctx.storage, OWNER).unwrap();
        assert_eq!(
            user_state.margin,
            prior_margin.checked_add(treasury_amount).unwrap(),
        );
    }
}
