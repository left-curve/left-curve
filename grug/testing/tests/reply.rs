use {
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{Addr, Coin, Coins, Empty, QuerierExt, ReplyOn, ResultExt},
    grug_vm_rust::ContractBuilder,
    replier::{ExecuteMsg, QueryDataRequest, ReplyMsg},
    test_case::test_case,
};

mod replier {
    use {
        grug_storage::Set,
        grug_types::{
            Coins, Empty, ImmutableCtx, Json, JsonSerExt, Message, MutableCtx, Order, QueryRequest,
            ReplyOn, Response, StdError, StdResult, SubMessage, SubMsgResult, SudoCtx,
        },
        serde::{Deserialize, Serialize},
    };

    #[derive(Serialize, Deserialize)]
    pub enum ReplyMsg {
        Ok(ExecuteMsg),
        Fail(ExecuteMsg),
    }

    #[derive(Serialize, Deserialize)]
    pub enum ExecuteMsg {
        /// Insert the given string into storage. Should be successful.
        Ok { deep: String },
        /// Intentionally fail with the given error message.
        Fail { err: String },
        /// Insert the given string into storage; then, call self with the given
        /// execute message.
        Perform {
            deep: String,
            // Must be boxed due to being a recursive type.
            next: Box<ExecuteMsg>,
            reply_on: ReplyOn,
        },
    }

    impl ExecuteMsg {
        pub fn ok<T>(deep: T) -> Self
        where
            T: Into<String>,
        {
            Self::Ok { deep: deep.into() }
        }

        pub fn fail<E>(err: E) -> Self
        where
            E: Into<String>,
        {
            Self::Fail { err: err.into() }
        }

        pub fn perform<T>(deep: T, next: ExecuteMsg, reply_on: ReplyOn) -> Self
        where
            T: Into<String>,
        {
            Self::Perform {
                deep: deep.into(),
                next: Box::new(next),
                reply_on,
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub enum QueryMsg {
        Data {},
    }

    pub struct QueryDataRequest {}

    impl QueryRequest for QueryDataRequest {
        type Message = QueryMsg;
        type Response = Vec<String>;
    }

    impl From<QueryDataRequest> for QueryMsg {
        fn from(_req: QueryDataRequest) -> Self {
            Self::Data {}
        }
    }

    pub const DEEPTHS: Set<String> = Set::new("s");

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
        match msg {
            ExecuteMsg::Fail { err } => {
                // We don't have a generic error as in CosmWasm, so use host
                // error to mock it.
                Err(StdError::host(err))
            },
            ExecuteMsg::Ok { deep } => {
                DEEPTHS.insert(ctx.storage, deep)?;

                Ok(Response::new())
            },
            ExecuteMsg::Perform {
                deep,
                next,
                reply_on,
            } => {
                DEEPTHS.insert(ctx.storage, deep)?;

                Ok(Response::new().add_submessage(SubMessage {
                    msg: Message::execute(ctx.contract, &*next, Coins::new())?,
                    reply_on,
                }))
            },
        }
    }

    pub fn query(ctx: ImmutableCtx, msg: QueryMsg) -> StdResult<Json> {
        match msg {
            QueryMsg::Data {} => {
                let res = DEEPTHS
                    .range(ctx.storage, None, None, Order::Ascending)
                    .collect::<StdResult<Vec<_>>>()?;
                res.to_json_value()
            },
        }
    }

    pub fn reply(ctx: SudoCtx, msg: ReplyMsg, res: SubMsgResult) -> StdResult<Response> {
        let msg = match (res.into_result(), msg) {
            (Result::Err(wee), ReplyMsg::Fail(execute_msg)) => {
                println!("replying with error: {wee}");
                execute_msg
            },
            (Result::Ok(_), ReplyMsg::Ok(execute_msg)) => execute_msg,
            _ => panic!("invalid reply"),
        };

        execute(
            MutableCtx {
                storage: ctx.storage,
                api: ctx.api,
                querier: ctx.querier,
                chain_id: ctx.chain_id,
                block: ctx.block,
                contract: ctx.contract,
                sender: ctx.contract,
                funds: Coins::new(),
            },
            msg,
        )
    }
}

fn setup() -> (TestSuite, TestAccounts, Addr) {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coin::new("usdc", 100_000).unwrap())
        .add_account("sender", Coins::new())
        .set_owner("owner")
        .build();

    let replier_code = ContractBuilder::new(Box::new(replier::instantiate))
        .with_execute(Box::new(replier::execute))
        .with_query(Box::new(replier::query))
        .with_reply(Box::new(replier::reply))
        .build();

    let replier_addr = suite
        .upload_and_instantiate(
            &mut accounts["owner"],
            replier_code,
            &Empty {},
            "salt",
            Some("label"),
            None,
            Coins::default(),
        )
        .should_succeed()
        .address;

    (suite, accounts, replier_addr)
}

// ------------------------------ ReplyOn::Always ------------------------------
#[test_case(
    ExecuteMsg::perform(
       "1",
        ExecuteMsg::ok("2"),
        ReplyOn::always(
            &ReplyMsg::Ok(
                ExecuteMsg::ok("1.1"),
            ),
        )
        .unwrap()
    ),
    ["1", "2", "1.1"],
    false;
    "reply_always_1pe_2pe_reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::fail("execute deep 2 fail"),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::ok("1.1"),
            ),
        )
        .unwrap()
    ),
    ["1", "1.1"],
    false;
    "reply_always_1pe_2fail_reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::ok("2"),
        ReplyOn::always(
            &ReplyMsg::Ok(
                ExecuteMsg::fail("reply deep 1 fail"),
            ),
        )
        .unwrap()
    ),
    [],
    true;
    "reply_always_1pe_2ok_reply_1.1fail"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::perform(
            "2",
            ExecuteMsg::perform(
                "3",
                ExecuteMsg::fail("execute deep 4 fail"),
                ReplyOn::Never,
            ),
            ReplyOn::Never,
        ),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::ok("1.1"),
            ),
        )
        .unwrap()
    ),
    ["1", "1.1"],
    false;
    "reply_always_1pe_2pe_3pe_4fail_1reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::perform(
            "2",
            ExecuteMsg::fail("execute deep 3 fail"),
            ReplyOn::Never,
        ),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::perform(
                    "1.1",
                    ExecuteMsg::ok("2.1"),
                    ReplyOn::Never,
                ),
            ),
        )
        .unwrap(),
    ),
    ["1", "1.1", "2.1"],
    false;
    "reply_always_1pe_2pe_3f_1reply_1.1pe_2.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::perform(
            "2",
            ExecuteMsg::perform(
                "3",
                ExecuteMsg::fail("execute deep 4 fail"),
                ReplyOn::Never,
            ),
            ReplyOn::always(
                &ReplyMsg::Fail(
                    ExecuteMsg::ok("3.2"),
                ),
            )
            .unwrap(),
        ),
        ReplyOn::always(
            &ReplyMsg::Ok(
                ExecuteMsg::perform(
                    "1.1",
                    ExecuteMsg::ok("2.1"),
                    ReplyOn::Never,
                ),
            ),
        )
        .unwrap(),
    ),
    ["1", "2", "3.2", "1.1", "2.1"],
    false;
    "reply_always_1pe_2pe_3_pe_4f_2reply_3.2ok_1reply_1.1pe_2.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::perform(
            "2",
            ExecuteMsg::perform(
                "3",
                ExecuteMsg::fail("execute deep 4 fail"),
                ReplyOn::Never,
            ),
            ReplyOn::always(
                &ReplyMsg::Fail(
                    ExecuteMsg::fail("reply deep 2 fail"),
                ),
            )
            .unwrap(),
        ),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::perform(
                    "1.1",
                    ExecuteMsg::ok("2.1"),
                    ReplyOn::Never,
                ),
            )
        )
        .unwrap(),
    ),
    ["1", "1.1", "2.1"],
    false;
    "reply_always_1pe_2pe_3pe_4fail_2reply_3.2fail_1reply_1.1pe_2.1ok"
)]
// ----------------------------- ReplyOn::Success ------------------------------
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::ok("2"),
        ReplyOn::success(
            &ReplyMsg::Ok(
                ExecuteMsg::ok("1.1"),
            ),
        )
        .unwrap(),
    ),
    ["1", "2", "1.1"],
    false;
    "reply_success_1pe_2ok_reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::ok("2"),
        ReplyOn::success(
            &ReplyMsg::Ok(
                ExecuteMsg::fail("reply deep 1 fail"),
            ),
        )
        .unwrap(),
    ),
    [],
    true;
    "reply_success_1pe_2ok_reply_1.1fail"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::fail("execute deep 2 fail"),
        ReplyOn::success(
            &ReplyMsg::Ok(
                ExecuteMsg::ok("1.1"),
            ),
        )
        .unwrap(),
    ),
    [],
    true;
    "reply_success_1pe_2fail_reply_1.1ok"
)]
// ------------------------------ ReplyOn::Error -------------------------------
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::fail("execute deep 2 fail"),
        ReplyOn::error(
            &ReplyMsg::Fail(
                ExecuteMsg::ok("1.1"),
            ),
        )
        .unwrap(),
    ),
    ["1", "1.1"],
    false;
    "reply_error_1pe_2fail_reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::fail("execute deep 2 fail"),
        ReplyOn::error(
            &ReplyMsg::Fail(
                ExecuteMsg::fail("reply deep 1 fail"),
            ),
        )
        .unwrap(),
    ),
    [],
    true;
    "reply_error_1pe_2fail_reply_1.1fail"
)]
#[test_case(
    ExecuteMsg::perform(
        "1",
        ExecuteMsg::ok("2"),
        ReplyOn::error(
            &ReplyMsg::Ok(
                ExecuteMsg::fail("reply deep 1 fail"),
            ),
        )
        .unwrap(),
    ),
    ["1", "2"],
    false;
    "reply_error_1pe_2ok_reply_1.1fail"
)]
fn reply<const S: usize>(msg: ExecuteMsg, mut data: [&str; S], should_tx_fail: bool) {
    let (mut suite, mut accounts, replier_addr) = setup();

    let result = suite.execute(&mut accounts["owner"], replier_addr, &msg, Coins::default());

    if should_tx_fail {
        result.should_fail();
    } else {
        result.should_succeed();
    }

    data.sort();

    suite
        .query_wasm_smart(replier_addr, QueryDataRequest {})
        .should_succeed_and_equal(data);
}
