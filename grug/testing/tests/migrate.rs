use {
    grug_testing::{TestBuilder, UploadAndInstantiateOutcomeSuccess},
    grug_types::{Coins, Empty, QuerierExt, ResultExt},
    grug_vm_rust::ContractBuilder,
    tester::{MigrateMsg, QueryV1, QueryV2RequestV1, QueryV2RequestV2},
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

    pub fn migrate_v2(ctx: SudoCtx, msg: MigrateMsg) -> StdResult<Response> {
        match msg {
            MigrateMsg::Ok => {
                ITEM_V2.save(ctx.storage, &"v2".to_string())?;
                Ok(Response::new())
            },
            MigrateMsg::Fail => Err(StdError::host("migrate failed".to_string())),
        }
    }

    pub fn query_v1(ctx: ImmutableCtx, _msg: QueryV1) -> StdResult<Json> {
        ITEM_V1.load(ctx.storage)?.to_json_value()
    }

    pub fn query_v2(ctx: ImmutableCtx, msg: QueryV2) -> StdResult<Json> {
        match msg {
            QueryV2::V1 => ITEM_V1.load(ctx.storage)?.to_json_value(),
            QueryV2::V2 => ITEM_V2.load(ctx.storage)?.to_json_value(),
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct QueryV1;

    impl QueryRequest for QueryV1 {
        type Message = Self;
        type Response = String;
    }

    #[derive(Serialize, Deserialize)]
    pub enum QueryV2 {
        V1,
        V2,
    }

    #[derive(Serialize, Deserialize)]
    pub struct QueryV2RequestV1;

    impl QueryRequest for QueryV2RequestV1 {
        type Message = QueryV2;
        type Response = String;
    }

    impl From<QueryV2RequestV1> for QueryV2 {
        fn from(_: QueryV2RequestV1) -> Self {
            QueryV2::V1
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct QueryV2RequestV2;

    impl QueryRequest for QueryV2RequestV2 {
        type Message = QueryV2;
        type Response = String;
    }

    impl From<QueryV2RequestV2> for QueryV2 {
        fn from(_: QueryV2RequestV2) -> Self {
            QueryV2::V2
        }
    }
}

#[test]
fn migrate() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::new())
        .set_owner("owner")
        .build();

    let v1 = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query_v1))
        .build();

    let v2 = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query_v2))
        .with_migrate(Box::new(tester::migrate_v2))
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
            .query_contract(contract)
            .should_succeed_and(|info| info.code_hash == v1_code_hash);
    }

    // Ok migrate
    {
        // Query current contract
        suite
            .query_wasm_smart(contract, QueryV1)
            .should_succeed_and_equal("v1");

        suite
            .migrate(&mut admin, contract, v2_code_hash, &MigrateMsg::Ok)
            .should_succeed();

        // Check the code_hash is still the old one
        suite
            .query_contract(contract)
            .should_succeed_and(|info| info.code_hash == v2_code_hash);

        suite
            .query_wasm_smart(contract, QueryV2RequestV1)
            .should_succeed_and_equal("v1");

        suite
            .query_wasm_smart(contract, QueryV2RequestV2)
            .should_succeed_and_equal("v2");
    }
}

#[test]
fn migrate_no_admin() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::new())
        .set_owner("owner")
        .build();

    let v1 = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query_v1))
        .build();

    let v2 = ContractBuilder::new(Box::new(tester::instantiate))
        .with_query(Box::new(tester::query_v2))
        .with_migrate(Box::new(tester::migrate_v2))
        .build();

    let mut admin = accounts.remove("owner").unwrap();

    let UploadAndInstantiateOutcomeSuccess {
        address: contract, ..
    } = suite
        .upload_and_instantiate(
            &mut admin,
            v1,
            &Empty {},
            "salt",
            None::<String>,
            None,
            Coins::default(),
        )
        .should_succeed();

    let v2_code_hash = suite.upload(&mut admin, v2).should_succeed().code_hash;

    // Admin not set, migrate fails
    suite
        .migrate(&mut admin, contract, v2_code_hash, &MigrateMsg::Ok)
        .should_fail_with_error("sender does not have permission to perform this action");
}
