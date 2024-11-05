use {
    grug_testing::TestBuilder,
    grug_types::{Addr, Coins, ConfigUpdates, Empty, Message, ResultExt},
    grug_vm_rust::ContractBuilder,
    std::collections::BTreeMap,
    tester::QueryConfigRequest,
};

mod tester {
    use {
        grug_storage::Item,
        grug_types::{
            Config, Empty, ImmutableCtx, Json, JsonSerExt, MutableCtx, QueryRequest, Response,
            StdResult,
        },
        serde::{Deserialize, Serialize},
    };

    const CFG: Item<Config> = Item::new("cfg");

    pub fn instantiate(ctx: MutableCtx, _: Empty) -> StdResult<Response> {
        let cfg = ctx.querier.query_config()?;
        CFG.save(ctx.storage, &cfg)?;
        Ok(Response::new().add_attribute("q-cfg", cfg.to_json_string()?))
    }

    pub fn query(ctx: ImmutableCtx, _: QueryMsg) -> StdResult<Json> {
        let cfg = CFG.load(ctx.storage)?;
        cfg.to_json_value()
    }

    #[derive(Serialize, Deserialize)]
    pub enum QueryMsg {
        Config,
    }

    impl From<QueryConfigRequest> for QueryMsg {
        fn from(_: QueryConfigRequest) -> Self {
            Self::Config
        }
    }

    #[derive(Serialize)]
    pub struct QueryConfigRequest {}

    impl QueryRequest for QueryConfigRequest {
        type Message = QueryMsg;
        type Response = Config;
    }
}

#[test]
fn update_cfg() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("rhaki", Coins::new())
        .add_account("larry", Coins::new())
        .set_owner("rhaki")
        .build();

    // Get current owner (sanity check, it should be "rhaki").
    let cfg = suite.query_config().should_succeed();
    let old_owner = cfg.owner;
    assert_eq!(old_owner, accounts["rhaki"].address);

    // Upload the tester contract code.
    let code = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query))
        .build();
    let code_hash = suite
        .upload(&mut accounts["rhaki"], code)
        .should_succeed()
        .code_hash;

    // Change owner and init tester. During init tester contract query the config and save it.
    // The cfg should not be changed until end of block.
    let new_owner_addr = accounts["larry"].address;
    let tester_addr = Addr::derive(accounts["rhaki"].address, code_hash, b"salt");

    suite
        .send_messages(&mut accounts["rhaki"], vec![
            Message::configure(
                ConfigUpdates {
                    owner: Some(new_owner_addr),
                    bank: Some(cfg.bank),
                    taxman: Some(cfg.taxman),
                    cronjobs: Some(cfg.cronjobs),
                    permissions: Some(cfg.permissions),
                },
                BTreeMap::default(),
            ),
            Message::instantiate(
                code_hash,
                &Empty {},
                "salt",
                None::<&str>,
                None,
                Coins::default(),
            )
            .unwrap(),
        ])
        .should_succeed();

    // Query the cfg from the tester contract.
    let cfg = suite
        .query_wasm_smart(tester_addr, QueryConfigRequest {})
        .should_succeed();

    assert_eq!(cfg.owner, old_owner);

    // check if the owner is changed in the app.
    let cfg = suite.query_config().should_succeed();

    assert_eq!(cfg.owner, new_owner_addr);
}
