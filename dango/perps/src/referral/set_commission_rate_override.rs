use {
    crate::state::COMMISSION_RATE_OVERRIDES,
    anyhow::ensure,
    dango_types::{account_factory::UserIndex, perps::CommissionRate},
    grug::{MutableCtx, Op, QuerierExt, Response},
};

/// Set or remove a commission rate override for a user.
///
/// Only callable by the chain owner.
pub fn set_commission_rate_override(
    ctx: MutableCtx,
    user: UserIndex,
    commission_rate: Op<CommissionRate>,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    match commission_rate {
        Op::Insert(rate) => {
            ensure!(
                !rate.is_negative() && rate <= CommissionRate::ONE,
                "commission rate must be in [0, 1], found: {rate}"
            );
            COMMISSION_RATE_OVERRIDES.save(ctx.storage, user, &rate)?;
        },
        Op::Delete => {
            COMMISSION_RATE_OVERRIDES.remove(ctx.storage, user);
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
            Addr, Coins, Config, Duration, MockContext, MockQuerier, Permission, Permissions,
            ResultExt,
        },
        std::collections::BTreeMap,
    };

    const OWNER: Addr = Addr::mock(1);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(100),
            taxman: Addr::mock(101),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Everybody,
                instantiate: Permission::Everybody,
            },
            max_orphan_age: Duration::from_seconds(1000),
        }
    }

    #[test]
    fn negative_commission_rate_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_commission_rate_override(
            ctx.as_mutable(),
            42,
            Op::Insert(CommissionRate::new_percent(-10)),
        )
        .should_fail_with_error("commission rate must be in [0, 1]");
    }

    #[test]
    fn commission_rate_above_one_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_commission_rate_override(
            ctx.as_mutable(),
            42,
            Op::Insert(CommissionRate::new_percent(150)),
        )
        .should_fail_with_error("commission rate must be in [0, 1]");
    }

    #[test]
    fn valid_commission_rate_accepted() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_commission_rate_override(
            ctx.as_mutable(),
            42,
            Op::Insert(CommissionRate::new_percent(25)),
        )
        .should_succeed();

        let stored = COMMISSION_RATE_OVERRIDES.load(&ctx.storage, 42).unwrap();
        assert_eq!(stored, CommissionRate::new_percent(25));
    }

    #[test]
    fn zero_commission_rate_accepted() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_commission_rate_override(ctx.as_mutable(), 42, Op::Insert(CommissionRate::ZERO))
            .should_succeed();
    }

    #[test]
    fn one_commission_rate_accepted() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        set_commission_rate_override(ctx.as_mutable(), 42, Op::Insert(CommissionRate::ONE))
            .should_succeed();
    }
}
