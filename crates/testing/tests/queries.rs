use {
    grug_testing::TestBuilder,
    grug_types::{Coins, Empty},
    grug_vm_rust::ContractBuilder,
};

mod query_maker {
    use {
        anyhow::ensure,
        grug_types::{Denom, Empty, MutableCtx, Number, Query, Response, Uint256},
    };

    pub fn instantiate(ctx: MutableCtx, _msg: Empty) -> anyhow::Result<Response> {
        // Attempt to make a multi query.
        let [res1, res2] = ctx.querier.query_multi([
            Query::Balance {
                address: ctx.contract,
                denom: Denom::new("uusdc")?,
            },
            Query::Supply {
                denom: Denom::new("uusdc")?,
            },
        ])?;

        ensure!(res1.as_balance().amount.is_zero());
        ensure!(res2.as_supply().amount == Uint256::from(123_u128));

        Ok(Response::new())
    }
}

#[test]
fn handling_multi_query() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("larry", Coins::one("uusdc", 123_u128)?)?
        .set_chain_id("kebab")
        .set_owner("larry")?
        .build()?;

    let query_maker_code = ContractBuilder::new(Box::new(query_maker::instantiate)).build();

    // If the contract successfully deploys, the multi query must have worked.
    suite.upload_and_instantiate(
        &accounts["larry"],
        query_maker_code,
        "query_maker",
        &Empty {},
        Coins::new(),
    )?;

    Ok(())
}
