use {
    crate::state::FEE_SHARE_RATIO,
    anyhow::ensure,
    dango_types::{account_factory::UserIndex, perps::FeeShareRatio},
    grug::{MutableCtx, QuerierExt, Response},
};

/// Forcibly set a user's fee share ratio.
///
/// Only callable by the chain owner. Bypasses the maximum ratio cap,
/// volume requirement, and only-increase restriction that apply to the
/// normal [`set_fee_share_ratio`] path.
pub fn force_set_fee_share_ratio(
    ctx: MutableCtx,
    user: UserIndex,
    share_ratio: FeeShareRatio,
) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "you don't have the right, O you don't have the right"
    );

    FEE_SHARE_RATIO.save(ctx.storage, user, &share_ratio)?;

    Ok(Response::new())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        grug::{Addr, Coins, Config, Duration, MockContext, MockQuerier, Permission, Permissions, ResultExt},
        std::collections::BTreeMap,
    };

    const OWNER: Addr = Addr::mock(1);
    const NOT_OWNER: Addr = Addr::mock(2);

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
    fn only_owner_can_force_set() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(NOT_OWNER)
            .with_funds(Coins::default());

        force_set_fee_share_ratio(ctx.as_mutable(), 42, FeeShareRatio::new_percent(25))
            .should_fail_with_error("you don't have the right");
    }
}
