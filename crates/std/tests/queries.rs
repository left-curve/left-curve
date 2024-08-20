use {
    grug::{Coins, ContractBuilder, NonZero, TestBuilder, Uint128},
    super_smart_querier::{Data, DataRequest},
};

mod super_smart_querier {
    use grug::{to_json_value, ImmutableCtx, Item, Json, MutableCtx, Response, StdResult, Uint128};

    const DATA: Item<Data> = Item::new("data");

    #[grug::derive(serde, borsh)]
    pub struct Data {
        pub foo: String,
        pub bar: Uint128,
    }

    #[grug::derive(serde)]
    #[derive(grug::Query)]
    pub enum QueryMsg {
        #[returns(Data)]
        Data {},
    }

    pub fn instantiate(ctx: MutableCtx, data: Data) -> StdResult<Response> {
        DATA.save(ctx.storage, &data)?;

        Ok(Response::new())
    }

    pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
        match msg {
            QueryMsg::Data {} => {
                let data = &DATA.load(ctx.storage)?;
                to_json_value(&data)
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
            &Data {
                foo: "rhaki".to_string(),
                bar: Uint128::new(123),
            },
            Coins::new(),
        )
        .unwrap();

    // Here, the compiler should be able to infer the type of the response as
    // `Data` based on the request type `DataRequest`.
    let res = suite
        .query_wasm_super_smart(contract, DataRequest {})
        .should_succeed();

    assert_eq!(res.foo, "rhaki");
    assert_eq!(res.bar.number(), 123);
}
