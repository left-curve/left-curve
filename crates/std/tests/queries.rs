use {
    grug::{Addr, Coins, ContractBuilder, Empty, NonZero, TestBuilder},
    super_smart_querier::{QueryFooRequest, QueryFuzzRequest},
};

mod super_smart_querier {
    use grug::{to_json_value, Addr, Empty, ImmutableCtx, Json, MutableCtx, Response, StdResult};

    #[grug::derive(serde)]
    #[derive(grug::Query)]
    pub enum QueryMsg {
        #[returns(String)]
        Foo { bar: u64 },
        #[returns(Addr)]
        Fuzz(u8),
    }

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn query(_ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
        match msg {
            QueryMsg::Foo { bar } => {
                let bar = bar.to_string();
                to_json_value(&bar)
            },
            QueryMsg::Fuzz(buzz) => {
                let buzz = Addr::mock(buzz);
                to_json_value(&buzz)
            },
        }
    }
}

#[test]
fn query_super_smart() {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("larry", Coins::one("uusdc", NonZero::new(123_u128)))
        .unwrap()
        .set_chain_id("kebab")
        .set_owner("larry")
        .unwrap()
        .build()
        .unwrap();

    let code = ContractBuilder::new(Box::new(super_smart_querier::instantiate))
        .with_query(Box::new(super_smart_querier::query))
        .build();

    let (_, contract) = suite
        .upload_and_instantiate(
            &accounts["larry"],
            code,
            "contract",
            &Empty {},
            Coins::new(),
        )
        .unwrap();

    // Here, the compiler should be able to infer the type of the response as
    // `String` based on the request type `QueryFooRequest`.
    suite
        .query_wasm_super_smart(contract, QueryFooRequest { bar: 12345 })
        .should_succeed_and_equal(12345.to_string());

    // Similarly, the compiler should be able to infer the type of response as
    // `Addr` based on the request type `QueryBarRequest`.
    suite
        .query_wasm_super_smart(contract, QueryFuzzRequest(123))
        .should_succeed_and_equal(Addr::mock(123));
}
