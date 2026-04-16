use {
    crate::state::FEE_RATE_OVERRIDES,
    anyhow::ensure,
    dango_types::Dimensionless,
    grug::{Addr, MutableCtx, Op, QuerierExt, Response},
};

pub fn set_fee_rate_override(
    ctx: MutableCtx,
    user: Addr,
    maker_taker_fee_rates: Op<(Dimensionless, Dimensionless)>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    match maker_taker_fee_rates {
        Op::Insert((maker_fee_rate, taker_fee_rate)) => {
            ensure!(
                (Dimensionless::ZERO..=Dimensionless::ONE).contains(&maker_fee_rate),
                "invalid maker fee rate: {maker_fee_rate}! must be within [0, 1]"
            );

            ensure!(
                (Dimensionless::ZERO..=Dimensionless::ONE).contains(&taker_fee_rate),
                "invalid taker fee rate: {taker_fee_rate}! must be within [0, 1]"
            );

            FEE_RATE_OVERRIDES.save(ctx.storage, user, &(maker_fee_rate, taker_fee_rate))?;
        },
        Op::Delete => {
            FEE_RATE_OVERRIDES.remove(ctx.storage, user);
        },
    }

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{
            Coins, Config, Duration, MockContext, MockQuerier, Permission, Permissions, ResultExt,
        },
        std::collections::BTreeMap,
    };

    const OWNER: Addr = Addr::mock(0);
    const NON_OWNER: Addr = Addr::mock(1);
    const USER: Addr = Addr::mock(42);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(100),
            taxman: Addr::mock(101),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Nobody,
                instantiate: Permission::Nobody,
            },
            max_orphan_age: Duration::from_seconds(0),
        }
    }

    /// A `(maker, taker)` rate pair that passes validation. Individual tests
    /// mutate one side to drive it out of range.
    fn valid_rates() -> (Dimensionless, Dimensionless) {
        (
            Dimensionless::new_permille(1), // 0.1% maker
            Dimensionless::new_permille(5), // 0.5% taker
        )
    }

    // ------------------------------ authorization ------------------------------

    #[test]
    fn non_owner_insert_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(NON_OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(valid_rates()))
            .should_fail_with_error("you don't have the right");
    }

    #[test]
    fn non_owner_delete_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(NON_OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Delete)
            .should_fail_with_error("you don't have the right");
    }

    // ----------------------------- happy path (insert) -------------------------

    #[test]
    fn valid_override_accepted_and_stored() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());
        let rates = valid_rates();

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(rates)).should_succeed();

        let stored = FEE_RATE_OVERRIDES.load(&ctx.storage, USER).unwrap();
        assert_eq!(stored, rates);
    }

    #[test]
    fn zero_maker_and_taker_accepted() {
        // Inclusive lower boundary of [0, 1].
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());
        let rates = (Dimensionless::ZERO, Dimensionless::ZERO);

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(rates)).should_succeed();

        let stored = FEE_RATE_OVERRIDES.load(&ctx.storage, USER).unwrap();
        assert_eq!(stored, rates);
    }

    #[test]
    fn one_maker_and_taker_accepted() {
        // Inclusive upper boundary of [0, 1].
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());
        let rates = (Dimensionless::ONE, Dimensionless::ONE);

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(rates)).should_succeed();

        let stored = FEE_RATE_OVERRIDES.load(&ctx.storage, USER).unwrap();
        assert_eq!(stored, rates);
    }

    #[test]
    fn insert_overwrites_previous_override() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());
        let first = (
            Dimensionless::new_permille(1),
            Dimensionless::new_permille(5),
        );
        let second = (
            Dimensionless::new_permille(2),
            Dimensionless::new_permille(10),
        );

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(first)).should_succeed();
        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(second)).should_succeed();

        let stored = FEE_RATE_OVERRIDES.load(&ctx.storage, USER).unwrap();
        assert_eq!(stored, second);
    }

    #[test]
    fn overrides_are_per_user() {
        // Setting an override for one user must not touch another user's entry.
        let other: Addr = Addr::mock(43);
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());
        let rates = valid_rates();

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(rates)).should_succeed();

        assert!(FEE_RATE_OVERRIDES.has(&ctx.storage, USER));
        assert!(!FEE_RATE_OVERRIDES.has(&ctx.storage, other));
    }

    // ---------------------------- validation rejections ------------------------

    #[test]
    fn negative_maker_fee_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(
            ctx.as_mutable(),
            USER,
            Op::Insert((Dimensionless::new_raw(-1), Dimensionless::new_permille(5))),
        )
        .should_fail_with_error("invalid maker fee rate");
    }

    #[test]
    fn maker_fee_above_one_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(
            ctx.as_mutable(),
            USER,
            Op::Insert((Dimensionless::new_int(2), Dimensionless::new_permille(5))),
        )
        .should_fail_with_error("invalid maker fee rate");
    }

    #[test]
    fn negative_taker_fee_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(
            ctx.as_mutable(),
            USER,
            Op::Insert((Dimensionless::new_permille(1), Dimensionless::new_raw(-1))),
        )
        .should_fail_with_error("invalid taker fee rate");
    }

    #[test]
    fn taker_fee_above_one_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(
            ctx.as_mutable(),
            USER,
            Op::Insert((Dimensionless::new_permille(1), Dimensionless::new_int(2))),
        )
        .should_fail_with_error("invalid taker fee rate");
    }

    #[test]
    fn rejected_override_leaves_state_untouched() {
        // An invalid rate on a user with an existing override must not
        // overwrite that entry. Validation runs before state mutation.
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());
        let prior = valid_rates();

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(prior)).should_succeed();

        set_fee_rate_override(
            ctx.as_mutable(),
            USER,
            Op::Insert((Dimensionless::new_int(2), Dimensionless::new_permille(5))),
        )
        .should_fail_with_error("invalid maker fee rate");

        let stored = FEE_RATE_OVERRIDES.load(&ctx.storage, USER).unwrap();
        assert_eq!(stored, prior);
    }

    // --------------------------------- delete ---------------------------------

    #[test]
    fn delete_removes_existing_override() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Insert(valid_rates())).should_succeed();
        assert!(FEE_RATE_OVERRIDES.has(&ctx.storage, USER));

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Delete).should_succeed();
        assert!(!FEE_RATE_OVERRIDES.has(&ctx.storage, USER));
    }

    #[test]
    fn delete_nonexistent_override_is_noop() {
        // `Map::remove` on a missing key must not panic or error.
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_fee_rate_override(ctx.as_mutable(), USER, Op::Delete).should_succeed();
        assert!(!FEE_RATE_OVERRIDES.has(&ctx.storage, USER));
    }
}
