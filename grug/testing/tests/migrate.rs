use {
    grug_testing::{TestBuilder, UploadAndInstantiateOutcomeSuccess},
    grug_types::{Coins, Empty, ResultExt},
    grug_vm_rust::ContractBuilder,
    tester::{MigrateMsg, QueryV1Request, QueryV2Request},
};

mod tester {
    use {
        grug_storage::Item,
        grug_types::{
            Empty, ImmutableCtx, Json, JsonSerExt, MutableCtx, QueryRequest, Response, StdError,
            StdResult, SudoCtx,
        },
        serde::{Deserialize, Serialize},
    };

    #[derive(Serialize, Deserialize)]
    pub enum MigrateMsg {
        Ok,
        Fail,
    }

    pub const ITEM_V1: Item<String> = Item::new("storage_key_v1");
    pub const ITEM_V2: Item<String> = Item::new("storage_key_v2");

    pub fn instantiate(ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        ITEM_V1.save(ctx.storage, &"v1".to_string())?;
        Ok(Response::new())
    }

    pub fn migrate(ctx: SudoCtx, msg: MigrateMsg) -> StdResult<Response> {
        match msg {
            MigrateMsg::Ok => {
                ITEM_V2.save(ctx.storage, &"v2".to_string())?;
                Ok(Response::new())
            },
            MigrateMsg::Fail => Err(StdError::host("migrate failed".to_string())),
        }
    }

    pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
        match msg {
            QueryMsg::V1 => ITEM_V1.load(ctx.storage)?.to_json_value(),
            QueryMsg::V2 => ITEM_V2.load(ctx.storage)?.to_json_value(),
        }
    }

    #[derive(Serialize, Deserialize)]
    pub enum QueryMsg {
        V1,
        V2,
    }

    #[derive(Serialize, Deserialize)]
    pub struct QueryV1Request;

    impl QueryRequest for QueryV1Request {
        type Message = QueryMsg;
        type Response = String;
    }

    impl From<QueryV1Request> for QueryMsg {
        fn from(_: QueryV1Request) -> Self {
            QueryMsg::V1
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct QueryV2Request;

    impl QueryRequest for QueryV2Request {
        type Message = QueryMsg;
        type Response = String;
    }

    impl From<QueryV2Request> for QueryMsg {
        fn from(_: QueryV2Request) -> Self {
            QueryMsg::V2
        }
    }
}

#[test]
fn test() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::new())
        .set_owner("owner")
        .build();

    let v1 = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query))
        .build();

    let v2 = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query))
        .with_migrate(Box::new(tester::migrate))
        .build();

    let mut admin = accounts.remove("owner").unwrap();
    let mut attacker = accounts.remove("sender").unwrap();

    let admin_addr = admin.address;

    let UploadAndInstantiateOutcomeSuccess {
        address: contract,
        code_hash: v1_code_hash,
        ..
    } = suite
        .upload_and_instantiate(
            &mut admin,
            v1,
            &Empty {},
            "salt",
            None::<String>,
            Some(admin_addr),
            Coins::default(),
        )
        .should_succeed();

    let v2_code_hash = suite.upload(&mut admin, v2).should_succeed().code_hash;

    // Try migrate from non_owner
    {
        suite
            .migrate(&mut attacker, contract, v2_code_hash, &MigrateMsg::Fail)
            .should_fail_with_error("sender does not have permission to perform this action");
    }

    // Migrate but a failure happens during migrate execution
    {
        suite
            .migrate(&mut admin, contract, v2_code_hash, &MigrateMsg::Fail)
            .should_fail_with_error("host returned error: migrate failed");

        // Check the code_hash is still the old one
        suite
            .query_contract(&contract)
            .should_succeed_and(|info| info.code_hash == v1_code_hash);
    }

    // Ok migrate
    // Migrate but a failure happens during migrate execution
    {
        suite
            .migrate(&mut admin, contract, v2_code_hash, &MigrateMsg::Ok)
            .should_succeed();

        // Check the code_hash is still the old one
        suite
            .query_contract(&contract)
            .should_succeed_and(|info| info.code_hash == v2_code_hash);

        suite
            .query_wasm_smart(contract, QueryV1Request)
            .should_succeed_and_equal("v1");

        suite
            .query_wasm_smart(contract, QueryV2Request)
            .should_succeed_and_equal("v2");
    }
}
