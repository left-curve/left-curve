use {
    grug_testing::TestBuilder,
    grug_types::{Coins, Empty, ResultExt},
    grug_vm_rust::ContractBuilder,
};

mod query_maker {
    use {
        grug_math::{IsZero, Uint128},
        grug_types::{Denom, Empty, MutableCtx, QuerierExt, Query, Response, StdResult},
        std::str::FromStr,
    };

    pub fn instantiate(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        // Attempt to make a multi query.
        let [res1, res2] = ctx.querier.query_multi([
            Query::balance(ctx.contract, Denom::from_str("uusdc")?),
            Query::supply(Denom::from_str("uusdc")?),
        ])?;

        assert!(res1?.as_balance().amount.is_zero());
        assert_eq!(res2?.as_supply().amount, Uint128::new(123));

        Ok(Response::new())
    }
}

#[test]
fn handling_multi_query() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("larry", Coins::one("uusdc", 123).unwrap())
        .set_chain_id("kebab")
        .set_owner("larry")
        .build();

    let query_maker_code = ContractBuilder::new(Box::new(query_maker::instantiate)).build();

    // If the contract successfully deploys, the multi query must have worked.
    suite
        .upload_and_instantiate(
            &mut accounts["larry"],
            query_maker_code,
            &Empty {},
            "query_maker",
            Some("query_maker"),
            None,
            Coins::new(),
        )
        .should_succeed();
}
