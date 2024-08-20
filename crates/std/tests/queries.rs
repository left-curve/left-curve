use {
    grug::{Coins, ContractBuilder, Empty, NonZero, TestBuilder},
    super_smart_querier::{AskConfigRequest, ConfigHost, QueryMsgClient},
};

mod super_smart_querier {
    use grug::{
        to_json_value, Addr, Empty, ImmutableCtx, Item, Json, MutableCtx, Response, StdResult,
        Uint128,
    };

    const CONFIG: Item<ConfigHost> = Item::new("c");

    #[grug::derive(serde, borsh)]
    pub struct ConfigHost {
        pub owner: String,
        pub balance: Uint128,
    }

    #[grug::derive(serde)]
    #[derive(grug::Query)]
    pub enum QueryMsgHost {
        #[returns(ConfigHost)]
        Config {},
    }

    pub fn instantiate_host(ctx: MutableCtx, config: ConfigHost) -> StdResult<Response> {
        CONFIG.save(ctx.storage, &config)?;

        Ok(Response::new())
    }

    pub fn query_host(ctx: ImmutableCtx, msg: QueryMsgHost) -> StdResult<Json> {
        match msg {
            QueryMsgHost::Config {} => to_json_value(&CONFIG.load(ctx.storage)?),
        }
    }

    // ---- Client ----
    #[grug::derive(serde)]
    #[derive(grug::Query)]
    pub enum QueryMsgClient {
        #[returns(ConfigHost)]
        AskConfig { contract: Addr },
    }

    pub fn instantiate_client(_: MutableCtx, _: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn query_client(ctx: ImmutableCtx, msg: QueryMsgClient) -> StdResult<Json> {
        match msg {
            QueryMsgClient::AskConfig { contract } => {
                let request = ConfigRequest {};
                let response = ctx.querier.query_wasm_super_smart(contract, request)?;

                Ok(to_json_value(&response)?)
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

    let host_contract = ContractBuilder::new(Box::new(super_smart_querier::instantiate_host))
        .with_query(Box::new(super_smart_querier::query_host))
        .build();

    let client_contract = ContractBuilder::new(Box::new(super_smart_querier::instantiate_client))
        .with_query(Box::new(super_smart_querier::query_client))
        .build();

    // If the contract successfully deploys, the multi query must have worked.
    let (_, host_contract) = suite
        .upload_and_instantiate(
            &accounts["larry"],
            host_contract,
            "host_contract",
            &ConfigHost {
                owner: "rhaki".to_string(),
                balance: 123_u128.into(),
            },
            Coins::new(),
        )
        .unwrap();

    let (_, client_contract) = suite
        .upload_and_instantiate(
            &accounts["larry"],
            client_contract,
            "client_contract",
            &Empty {},
            Coins::new(),
        )
        .unwrap();

    // Standard query_wasm_smart on suite
    {
        let result: ConfigHost = suite
            .query_wasm_smart(client_contract, &QueryMsgClient::AskConfig {
                contract: host_contract,
            })
            .should_succeed();

        assert_eq!(result.owner, "rhaki");
        assert_eq!(result.balance, 123_u128.into());
    }

    {
        let result = suite
            .query_wasm_super_smart(client_contract, AskConfigRequest {
                contract: host_contract,
            })
            .should_succeed();

        assert_eq!(result.owner, "rhaki");
        assert_eq!(result.balance, 123_u128.into());
    }
}
