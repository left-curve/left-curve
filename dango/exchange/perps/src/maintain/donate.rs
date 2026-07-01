use {
    anyhow::ensure,
    dango_math::IsZero,
    dango_primitives::{MutableCtx, QuerierExt, Response},
    dango_types::perps::settlement_currency,
};

/// Accept a USDC donation to the perps contract.
///
/// The donated tokens stay in the perps contract's bank balance, covering the
/// shortfall between user liabilities and contract assets after the exploit.
pub fn donate(mut ctx: MutableCtx) -> anyhow::Result<Response> {
    // 1. Only the chain owner may donate.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the chain owner can donate"
    );

    // 2. Must attach a nonzero amount of USDC.

    ensure!(
        {
            let amount = ctx.funds.take(settlement_currency::DENOM.clone()).amount;
            amount.is_non_zero()
        },
        "nothing to donate"
    );

    // 3. Must not attach any other tokens.
    ensure!(ctx.funds.is_empty(), "unexpected tokens: {}", ctx.funds);

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_math::Uint128,
        dango_primitives::{
            Addr, Coins, Config, Denom, Duration, MockContext, MockQuerier, Permission,
            Permissions, ResultExt,
        },
        dango_types::constants::usdc,
        std::collections::BTreeMap,
    };

    const OWNER: Addr = Addr::mock(0);
    const NON_OWNER: Addr = Addr::mock(1);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(2),
            gas_token: dango_primitives::Denom::new_unchecked(["ugas"]),
            gas_fee_rate: Default::default(),
            gas_exemptions: Default::default(),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Nobody,
                instantiate: Permission::Nobody,
            },
            max_orphan_age: Duration::from_seconds(0),
        }
    }

    fn usdc_coins(whole: u128) -> Coins {
        Coins::one(usdc::DENOM.clone(), Uint128::new(whole * 1_000_000)).unwrap()
    }

    #[test]
    fn non_owner_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(NON_OWNER)
            .with_funds(usdc_coins(100));

        donate(ctx.as_mutable()).should_fail_with_error("only the chain owner can donate");
    }

    #[test]
    fn zero_amount_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(Coins::default());

        donate(ctx.as_mutable()).should_fail_with_error("nothing to donate");
    }

    #[test]
    fn wrong_denom_rejected() {
        let other_denom: Denom = "factory/other".parse().unwrap();
        let wrong_coins = Coins::one(other_denom, Uint128::new(100)).unwrap();

        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(wrong_coins);

        donate(ctx.as_mutable()).should_fail_with_error("nothing to donate");
    }

    #[test]
    fn owner_donates_usdc_succeeds() {
        let mut ctx = MockContext::new()
            .with_querier(MockQuerier::new().with_config(mock_config()))
            .with_sender(OWNER)
            .with_funds(usdc_coins(1_000));

        donate(ctx.as_mutable()).should_succeed();
    }
}
