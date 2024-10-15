use {
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{btree_map, Addr, Coins, Empty, ReplyOn},
    grug_vm_rust::ContractBuilder,
    replier::{BorrowedMapData, ExecuteMsg, MapData, QueryDataRequest, ReplyMsg},
    test_case::test_case,
};

mod replier {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        grug_storage::Map,
        grug_types::{
            Coins, Empty, GenericResult, ImmutableCtx, Json, JsonSerExt, Message, MutableCtx,
            Order, QueryRequest, ReplyOn, Response, StdError, StdResult, Storage, SubMessage,
            SubMsgResult, SudoCtx,
        },
        serde::{Deserialize, Serialize},
        std::collections::BTreeMap,
    };

    pub type MapData = BTreeMap<u64, String>;

    pub type BorrowedMapData<'a> = BTreeMap<u64, &'a str>;

    pub fn convert_map_data(data: BorrowedMapData) -> MapData {
        data.into_iter().map(|(k, v)| (k, v.to_string())).collect()
    }

    pub const SAVE_DATA: Map<u64, String> = Map::new("s");

    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    pub enum ReplyMsg {
        Fail(ExecuteMsg),
        Ok(ExecuteMsg),
    }

    #[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    pub enum ExecuteMsg {
        Ok {
            data: MapData,
        },
        Fail {
            err: String,
        },
        Perform {
            data: MapData,
            next: Box<ExecuteMsg>,
            reply_on: ReplyOn,
        },
    }

    #[derive(Serialize, Deserialize)]
    pub enum QueryMsg {
        Data(QueryDataRequest),
    }

    impl From<QueryDataRequest> for QueryMsg {
        fn from(msg: QueryDataRequest) -> Self {
            Self::Data(msg)
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct QueryDataRequest {}

    impl QueryRequest for QueryDataRequest {
        type Message = QueryMsg;
        type Response = MapData;
    }

    impl ExecuteMsg {
        pub fn ok(data: BorrowedMapData) -> Self {
            Self::Ok {
                data: convert_map_data(data),
            }
        }

        pub fn fail<E>(err: E) -> Self
        where
            E: Into<String>,
        {
            Self::Fail { err: err.into() }
        }

        pub fn perform(data: BorrowedMapData, next: ExecuteMsg, reply_on: ReplyOn) -> Self {
            Self::Perform {
                data: convert_map_data(data),
                next: Box::new(next),
                reply_on,
            }
        }
    }

    pub fn instantiate(_ctx: MutableCtx, _msg: Empty) -> StdResult<Response> {
        Ok(Response::new())
    }

    pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> StdResult<Response> {
        match msg {
            ExecuteMsg::Fail { err } => Err(StdError::host(err)),
            ExecuteMsg::Ok { data } => {
                save_data(ctx.storage, data)?;
                Ok(Response::new())
            },
            ExecuteMsg::Perform {
                data,
                next,
                reply_on,
            } => {
                save_data(ctx.storage, data)?;
                Ok(Response::new().add_submessage(SubMessage {
                    msg: Message::execute(ctx.contract, &*next, Coins::default())?,
                    reply_on,
                }))
            },
        }
    }

    pub fn query(ctx: ImmutableCtx, _msg: Empty) -> StdResult<Json> {
        SAVE_DATA
            .range(ctx.storage, None, None, Order::Ascending)
            .collect::<StdResult<MapData>>()?
            .to_json_value()
    }

    pub fn reply(ctx: SudoCtx, msg: ReplyMsg, res: SubMsgResult) -> StdResult<Response> {
        let msg = match (res, msg) {
            (GenericResult::Err(_), ReplyMsg::Fail(execute_msg)) => execute_msg,
            (GenericResult::Ok(_), ReplyMsg::Ok(execute_msg)) => execute_msg,
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
                funds: Coins::default(),
            },
            msg,
        )
    }

    fn save_data(storage: &mut dyn Storage, save_data: MapData) -> StdResult<()> {
        for (k, v) in save_data {
            SAVE_DATA.save(storage, k, &v)?;
        }

        Ok(())
    }
}

fn setup() -> (TestSuite, TestAccounts, Addr) {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
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
        .address;

    (suite, accounts, replier_addr)
}

#[test_case(
    ExecuteMsg::perform(
        btree_map!(1 => "1"),
        ExecuteMsg::ok(btree_map!(2 => "2")),
        ReplyOn::always(
            &ReplyMsg::Ok(
                ExecuteMsg::ok(btree_map!())
            )
        ).unwrap()
    ),
    btree_map!(1 => "1", 2 => "2");
    "reply_always_1pe_2pe_reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        btree_map!(1 => "1"),
        ExecuteMsg::fail("execute deep 2 fail"),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::ok(btree_map!(3 => "3"))
            )
        ).unwrap()
    ),
    btree_map!(1 => "1", 3 => "3");
    "reply_always_1pe_2fail_reply_1.1ok"

)]
#[test_case(
    ExecuteMsg::perform(
        btree_map!(1 => "1"),
        ExecuteMsg::ok(btree_map!(2 => "2")),
        ReplyOn::always(
            &ReplyMsg::Ok(
                ExecuteMsg::fail("reply deep 1 fail")
            )
        ).unwrap()
    ),
    btree_map!();
    "reply_always_1pe_2ok_reply_1.1fail"
)]
#[test_case(
    ExecuteMsg::perform(
        btree_map!(1 => "1"),
        ExecuteMsg::perform(
            btree_map!(2 => "2"),
            ExecuteMsg::perform(
                btree_map!(3 => "3"),
                ExecuteMsg::fail("execute deep 4 fail"),
                ReplyOn::Never,
            ),
            ReplyOn::Never,
        ),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::ok(btree_map!())
            )
        ).unwrap()
    ),
    btree_map!(1 => "1");
    "reply_always_1pe_2pe_3pe_4fail_1reply_1.1ok"
)]
#[test_case(
    ExecuteMsg::perform(
        btree_map!(1 => "1"),
        ExecuteMsg::perform(
            btree_map!(2 => "2"),
            ExecuteMsg::fail("execute deep 3 fail"),
            ReplyOn::Never,
        ),
        ReplyOn::always(
            &ReplyMsg::Fail(
                ExecuteMsg::perform(
                    btree_map!(11 => "1.1"),
                    ExecuteMsg::ok(btree_map!(21 => "2.1")),
                    ReplyOn::Never,
                ),
            )
        ).unwrap()
    ),
    btree_map!(1 => "1", 21 => "2.1", 31 => "3.1");
    "reply_always_1pe_2pe_3f_1reply_1.1pe_2.1ok"
)]
fn reply(msg: ExecuteMsg, data: BorrowedMapData) {
    let (mut suite, mut accounts, replier_addr) = setup();

    suite.execute(&mut accounts["owner"], replier_addr, &msg, Coins::default());

    let res: MapData = suite
        .query_wasm_smart(replier_addr, QueryDataRequest {})
        .unwrap();

    let borrowed: BorrowedMapData = res.iter().map(|(k, v)| (*k, v.as_str())).collect();

    assert_eq!(borrowed, data);
}
