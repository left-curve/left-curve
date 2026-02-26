use {
    crate::{PARAM, USER_STATES},
    anyhow::ensure,
    dango_types::perps::settlement_currency,
    grug::{IsZero, Message, MutableCtx, Number as _, NumberConst, Response, Uint128, coins},
};

pub fn claim(ctx: MutableCtx) -> anyhow::Result<Response> {
    let _param = PARAM.load(ctx.storage)?;
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    // Pop all unlocks whose cooldown has completed.
    // Unlocks are stored in chronological order so we pop from the front.
    let mut total_to_release = Uint128::ZERO;

    while let Some(unlock) = user_state.unlocks.front() {
        if unlock.end_time > ctx.block.timestamp {
            break;
        }

        total_to_release.checked_add_assign(unlock.amount_to_release)?;
        user_state.unlocks.pop_front();
    }

    ensure!(total_to_release.is_non_zero(), "nothing to claim");

    // Persist updated user state (or remove if empty).
    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.sender);
    } else {
        USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    }

    // Transfer settlement currency to the user.
    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        coins! { settlement_currency::DENOM.clone() => total_to_release },
    )?))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{PARAM, USER_STATES},
        dango_types::perps::{Param, Unlock, UserState},
        grug::{Addr, Coins, MockContext, ResultExt, Timestamp, Uint128},
        std::collections::VecDeque,
    };

    const USER: Addr = Addr::mock(1);

    /// Helper: save default `Param` to storage.
    fn init_param(storage: &mut dyn grug::Storage) {
        PARAM.save(storage, &Param::default()).unwrap();
    }

    /// Helper: build a `VecDeque` of unlocks from `(amount, end_time_secs)` pairs.
    fn unlocks_from(entries: &[(u128, u128)]) -> VecDeque<Unlock> {
        entries
            .iter()
            .map(|&(amount, secs)| Unlock {
                amount_to_release: Uint128::new(amount),
                end_time: Timestamp::from_seconds(secs),
            })
            .collect()
    }

    // ---- Test 1: no user state → error ----
    #[test]
    fn no_user_state_errors() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_param(&mut ctx.storage);

        claim(ctx.as_mutable()).should_fail_with_error("not found");
    }

    // ---- Test 2: all unlocks still pending → nothing to claim ----
    #[test]
    fn all_unlocks_pending() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default())
            .with_block_timestamp(Timestamp::from_seconds(50));

        init_param(&mut ctx.storage);
        USER_STATES
            .save(&mut ctx.storage, USER, &UserState {
                unlocks: unlocks_from(&[(1_000, 100), (2_000, 200)]),
                ..Default::default()
            })
            .unwrap();

        claim(ctx.as_mutable()).should_fail_with_error("nothing to claim");
    }

    // ---- Test 3: single matured unlock claimed ----
    #[test]
    fn single_matured_unlock() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default())
            .with_block_timestamp(Timestamp::from_seconds(150));

        init_param(&mut ctx.storage);
        USER_STATES
            .save(&mut ctx.storage, USER, &UserState {
                unlocks: unlocks_from(&[(1_000, 100), (2_000, 200)]),
                ..Default::default()
            })
            .unwrap();

        let res = claim(ctx.as_mutable()).unwrap();
        assert_eq!(res.submsgs.len(), 1);

        // One unlock remains (end_time=200 > now=150).
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.unlocks.len(), 1);
        assert_eq!(state.unlocks[0].amount_to_release, Uint128::new(2_000));
    }

    // ---- Test 4: all unlocks matured → all claimed, user state removed ----
    #[test]
    fn all_unlocks_matured_removes_user_state() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default())
            .with_block_timestamp(Timestamp::from_seconds(300));

        init_param(&mut ctx.storage);
        USER_STATES
            .save(&mut ctx.storage, USER, &UserState {
                unlocks: unlocks_from(&[(1_000, 100), (2_000, 200)]),
                ..Default::default()
            })
            .unwrap();

        let res = claim(ctx.as_mutable()).unwrap();
        assert_eq!(res.submsgs.len(), 1);

        // User state fully empty → removed.
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    // ---- Test 5: exact boundary — end_time == block timestamp is matured ----
    #[test]
    fn exact_boundary_is_matured() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default())
            .with_block_timestamp(Timestamp::from_seconds(100));

        init_param(&mut ctx.storage);
        USER_STATES
            .save(&mut ctx.storage, USER, &UserState {
                unlocks: unlocks_from(&[(500, 100)]),
                ..Default::default()
            })
            .unwrap();

        claim(ctx.as_mutable()).unwrap();

        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    // ---- Test 6: partial claim — middle unlocks matured ----
    #[test]
    fn partial_claim_keeps_remaining() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default())
            .with_block_timestamp(Timestamp::from_seconds(250));

        init_param(&mut ctx.storage);
        USER_STATES
            .save(&mut ctx.storage, USER, &UserState {
                unlocks: unlocks_from(&[(100, 50), (200, 150), (300, 250), (400, 350)]),
                ..Default::default()
            })
            .unwrap();

        claim(ctx.as_mutable()).unwrap();

        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        // First three are matured (50, 150, 250 <= 250), last one remains.
        assert_eq!(state.unlocks.len(), 1);
        assert_eq!(state.unlocks[0].amount_to_release, Uint128::new(400));
        assert_eq!(state.unlocks[0].end_time, Timestamp::from_seconds(350));
    }
}
