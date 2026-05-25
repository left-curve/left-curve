use {
    dango_testing::{ContractBuilder, TestOption, setup_test_naive},
    grug_types::{Coins, Empty, ResultExt},
};

mod query_maker {
    use {
        dango_types::constants::usdc,
        grug_math::IsZero,
        grug_types::{Empty, MutableCtx, QuerierExt, Query, Response, StdResult},
    };

    pub fn instantiate(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        // Attempt to make a multi query.
        let [res1, res2] = ctx.querier.query_multi([
            Query::balance(ctx.contract, usdc::DENOM.clone()),
            Query::supply(usdc::DENOM.clone()),
        ])?;

        assert!(res1?.into_balance().amount.is_zero());
        assert!(!res2?.into_supply().amount.is_zero());

        Ok(Response::new())
    }
}

#[tokio::test]
async fn handling_multi_query() {
    let (mut suite, mut accounts, ..) = setup_test_naive(TestOption::default());

    let query_maker_code = ContractBuilder::new(Box::new(query_maker::instantiate)).build();

    suite
        .upload_and_instantiate(
            &mut accounts.owner,
            query_maker_code,
            &Empty {},
            "query_maker",
            Some("query_maker"),
            None,
            Coins::new(),
        )
        .await
        .should_succeed();
}
