use {
    crate::{USER_STATES, execute::ORACLE},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        UsdValue,
        perps::{UserState, settlement_currency},
    },
    grug::{IsZero, Message, MutableCtx, Response, Timestamp, coins},
};

/// Claim vault unlocks that have completed their cooldown period.
/// Converts accumulated UsdValue unlock amounts to settlement currency tokens
/// at the current oracle price (floor-rounded).
///
/// Mutates: `USER_STATES` (pops matured unlocks, removes if empty).
///
/// Returns: `Response` with a transfer message.
pub fn claim(ctx: MutableCtx) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    // --------------------------- Business logic ---------------------------

    let total_usd_to_release = _claim(ctx.block.timestamp, &mut user_state)?;

    // Persist updated user state (or remove if empty).
    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.sender)?;
    } else {
        USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    }

    // Convert USD to settlement currency base units at current oracle price.
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    let quantity = total_usd_to_release.checked_div(settlement_currency_price)?;
    let total_to_release = quantity.into_base_floor(settlement_currency::DECIMAL)?;

    ensure!(
        total_to_release.is_non_zero(),
        "claimable amount rounds to zero tokens"
    );

    // Transfer settlement currency to the user.
    Ok(Response::new().add_message(Message::transfer(
        ctx.sender,
        coins! { settlement_currency::DENOM.clone() => total_to_release },
    )?))
}

/// Pop matured unlocks from the user state and return the total USD to release.
///
/// Mutates: `user_state.unlocks` (pops matured entries from the front).
///
/// Returns: total `UsdValue` to release.
fn _claim(current_time: Timestamp, user_state: &mut UserState) -> anyhow::Result<UsdValue> {
    let mut total_usd_to_release = UsdValue::ZERO;

    while let Some(unlock) = user_state.unlocks.front() {
        if unlock.end_time > current_time {
            break;
        }

        total_usd_to_release.checked_add_assign(unlock.amount_to_release)?;
        user_state.unlocks.pop_front();
    }

    ensure!(total_usd_to_release.is_non_zero(), "nothing to claim");

    Ok(total_usd_to_release)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{PARAM, USER_STATES},
        dango_types::perps::{Param, Unlock, UserState},
        grug::{Addr, MockContext, Timestamp},
        std::collections::VecDeque,
    };

    const USER: Addr = Addr::mock(1);

    /// Helper: save default `Param` to storage.
    fn init_param(storage: &mut dyn grug::Storage) {
        PARAM.save(storage, &Param::default()).unwrap();
    }

    /// Helper: build a `VecDeque` of unlocks from `(usd_amount, end_time_secs)` pairs.
    fn unlocks_from(entries: &[(i128, u128)]) -> VecDeque<Unlock> {
        entries
            .iter()
            .map(|&(amount, secs)| Unlock {
                amount_to_release: UsdValue::new_int(amount),
                end_time: Timestamp::from_seconds(secs),
            })
            .collect()
    }

    // ---- Test 1: no user state → error ----
    #[test]
    fn no_user_state_errors() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(grug::Coins::default());

        init_param(&mut ctx.storage);

        // _claim requires a loaded user_state, but we test that USER_STATES.load fails.
        assert!(USER_STATES.load(&ctx.storage, USER).is_err());
    }

    // ---- Test 2: all unlocks still pending → nothing to claim ----
    #[test]
    fn all_unlocks_pending() {
        let mut user_state = UserState {
            unlocks: unlocks_from(&[(1_000, 100), (2_000, 200)]),
            ..Default::default()
        };

        let err = _claim(Timestamp::from_seconds(50), &mut user_state).unwrap_err();
        assert!(err.to_string().contains("nothing to claim"));

        // Unlocks unchanged.
        assert_eq!(user_state.unlocks.len(), 2);
    }

    // ---- Test 3: single matured unlock claimed ----
    #[test]
    fn single_matured_unlock() {
        let mut user_state = UserState {
            unlocks: unlocks_from(&[(1_000, 100), (2_000, 200)]),
            ..Default::default()
        };

        let total = _claim(Timestamp::from_seconds(150), &mut user_state).unwrap();
        assert_eq!(total, UsdValue::new_int(1_000));

        // One unlock remains (end_time=200 > now=150).
        assert_eq!(user_state.unlocks.len(), 1);
        assert_eq!(
            user_state.unlocks[0].amount_to_release,
            UsdValue::new_int(2_000)
        );
    }

    // ---- Test 4: all unlocks matured → all claimed, user state empty ----
    #[test]
    fn all_unlocks_matured_removes_user_state() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(grug::Coins::default());

        init_param(&mut ctx.storage);

        let mut user_state = UserState {
            unlocks: unlocks_from(&[(1_000, 100), (2_000, 200)]),
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, USER, &user_state)
            .unwrap();

        let total = _claim(Timestamp::from_seconds(300), &mut user_state).unwrap();
        assert_eq!(total, UsdValue::new_int(3_000));

        // User state fully empty.
        assert!(user_state.is_empty());

        // Persist (the outer fn would do this).
        if user_state.is_empty() {
            USER_STATES.remove(&mut ctx.storage, USER).unwrap();
        }
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    // ---- Test 5: exact boundary — end_time == block timestamp is matured ----
    #[test]
    fn exact_boundary_is_matured() {
        let mut user_state = UserState {
            unlocks: unlocks_from(&[(500, 100)]),
            ..Default::default()
        };

        let total = _claim(Timestamp::from_seconds(100), &mut user_state).unwrap();
        assert_eq!(total, UsdValue::new_int(500));
        assert!(user_state.unlocks.is_empty());
    }

    // ---- Test 6: partial claim — middle unlocks matured ----
    #[test]
    fn partial_claim_keeps_remaining() {
        let mut user_state = UserState {
            unlocks: unlocks_from(&[(100, 50), (200, 150), (300, 250), (400, 350)]),
            ..Default::default()
        };

        let total = _claim(Timestamp::from_seconds(250), &mut user_state).unwrap();
        assert_eq!(total, UsdValue::new_int(600)); // 100 + 200 + 300

        // Last one remains.
        assert_eq!(user_state.unlocks.len(), 1);
        assert_eq!(
            user_state.unlocks[0].amount_to_release,
            UsdValue::new_int(400)
        );
        assert_eq!(user_state.unlocks[0].end_time, Timestamp::from_seconds(350));
    }
}
