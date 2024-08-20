use {
    grug_testing::TestBuilder,
    grug_types::{Coins, Empty, NonZero},
    grug_vm_rust::ContractBuilder,
    query_super_smart::{AskConfigRequest, ConfigHost, QueryMsgClient},
};

mod query_maker {
    use {
        anyhow::ensure,
        grug_types::{Empty, MutableCtx, Number, QueryRequest, Response, Uint256},
    };

    pub fn instantiate(ctx: MutableCtx, _msg: Empty) -> anyhow::Result<Response> {
        // Attempt to make a multi query.
        let [res1, res2, res3] = ctx.querier.query_multi([
            QueryRequest::Info {},
            QueryRequest::Balance {
                address: ctx.contract,
                denom: "uusdc".to_string(),
            },
            QueryRequest::Supply {
                denom: "uusdc".to_string(),
            },
        ])?;

        ensure!(res1.as_info().chain_id == "kebab");
        ensure!(res2.as_balance().amount.is_zero());
        ensure!(res3.as_supply().amount == Uint256::from(123_u128));

        Ok(Response::new())
    }
}

#[test]
fn handling_multi_query() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("larry", Coins::one("uusdc", NonZero::new(123_u128)))?
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

mod query_super_smart {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        grug_storage::Item,
        grug_types::{
            to_json_value, Addr, Empty, ImmutableCtx, Json, MutableCtx, Response, Uint128,
        },
        serde::{Deserialize, Serialize},
    };

    const CONFIG: Item<ConfigHost> = Item::new("c");

    #[derive(Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
    pub struct ConfigHost {
        pub owner: String,
        pub balance: Uint128,
    }

    #[derive(Serialize, Deserialize, grug_macros::Query)]
    #[query_path_override(::grug_types)]
    pub enum QueryMsgHost {
        #[returns(ConfigHost)]
        Config {},
    }

    pub fn instantiate_host(ctx: MutableCtx, config: ConfigHost) -> anyhow::Result<Response> {
        CONFIG.save(ctx.storage, &config)?;
        Ok(Response::new())
    }

    pub fn query_host(ctx: ImmutableCtx, msg: QueryMsgHost) -> anyhow::Result<Json> {
        match msg {
            QueryMsgHost::Config {} => Ok(to_json_value(&CONFIG.load(ctx.storage)?)?),
        }
    }

    // ---- Client ----
    #[derive(Serialize, Deserialize, grug_macros::Query)]
    #[query_path_override(::grug_types)]
    pub enum QueryMsgClient {
        #[returns(ConfigHost)]
        AskConfig { contract: Addr },
    }

    pub fn instantiate_client(_: MutableCtx, _: Empty) -> anyhow::Result<Response> {
        Ok(Response::new())
    }

    pub fn query_client(ctx: ImmutableCtx, msg: QueryMsgClient) -> anyhow::Result<Json> {
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
fn query_super_smart() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("larry", Coins::one("uusdc", NonZero::new(123_u128)))?
        .set_chain_id("kebab")
        .set_owner("larry")?
        .build()?;

    let host_contract = ContractBuilder::new(Box::new(query_super_smart::instantiate_host))
        .with_query(Box::new(query_super_smart::query_host))
        .build();

    let client_contract = ContractBuilder::new(Box::new(query_super_smart::instantiate_client))
        .with_query(Box::new(query_super_smart::query_client))
        .build();

    // If the contract successfully deploys, the multi query must have worked.
    let (_, host_contract) = suite.upload_and_instantiate(
        &accounts["larry"],
        host_contract,
        "host_contract",
        &ConfigHost {
            owner: "rhaki".to_string(),
            balance: 123_u128.into(),
        },
        Coins::new(),
    )?;

    let (_, client_contract) = suite.upload_and_instantiate(
        &accounts["larry"],
        client_contract,
        "client_contract",
        &Empty {},
        Coins::new(),
    )?;

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

    Ok(())
}
