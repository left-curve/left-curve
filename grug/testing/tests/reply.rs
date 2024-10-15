use {
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{btree_map, Addr, Coins, Empty, ReplyOn, ResultExt},
    grug_vm_rust::ContractBuilder,
    replier::{ExecuteMsg, MapData, ReplyMsg},
};

mod replier {
    use {
        borsh::{BorshDeserialize, BorshSerialize},
        grug_storage::Map,
        grug_types::{
            Coins, Empty, GenericResult, ImmutableCtx, Json, JsonSerExt, Message, MutableCtx,
            Order, ReplyOn, Response, StdError, StdResult, Storage, SubMessage, SubMsgResult,
            SudoCtx,
        },
        serde::{Deserialize, Serialize},
        std::collections::BTreeMap,
    };

    pub type MapData = BTreeMap<u64, String>;

    pub const SAVE_DATA: Map<u64, String> = Map::new("s");

    #[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    pub enum ReplyMsg {
        Fail(ExecuteMsg),
        Ok(ExecuteMsg),
    }

    #[derive(Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
    pub enum ExecuteMsg {
        Ok {
            data: Option<MapData>,
        },
        Fail {
            err: String,
        },
        Perform {
            data: Option<MapData>,
            next: Box<ExecuteMsg>,
            reply_on: ReplyOn,
        },
    }

    impl ExecuteMsg {
        pub fn ok(data: Option<MapData>) -> Self {
            Self::Ok { data }
        }

        pub fn fail<E>(err: E) -> Self
        where
            E: Into<String>,
        {
            Self::Fail { err: err.into() }
        }

        pub fn perform(data: Option<MapData>, next: ExecuteMsg, reply_on: ReplyOn) -> Self {
            Self::Perform {
                data,
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

    fn save_data(storage: &mut dyn Storage, save_data: Option<MapData>) -> StdResult<()> {
        if let Some(save_data) = save_data {
            for (k, v) in save_data {
                SAVE_DATA.save(storage, k, &v)?;
            }
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

#[test]
fn reply_always_ok_ok() {
    let (mut suite, mut accounts, replier_addr) = setup();

    accounts.get_mut("owner").unwrap();
    suite
        .execute(
            &mut accounts["owner"],
            replier_addr,
            &ExecuteMsg::perform(
                Some(btree_map!(1 => "one".to_string())),
                ExecuteMsg::ok(Some(btree_map!(2 => "two".to_string()))),
                ReplyOn::always(&ReplyMsg::Ok(ExecuteMsg::ok(None))).unwrap(),
            ),
            Coins::default(),
        )

    let res: MapData = suite.query_wasm(replier_addr, &Empty {}).unwrap();

    assert_eq!(
        res,
        btree_map!(1 => "one".to_string(), 2 => "two".to_string())
    );
}

#[test]
fn reply_always_ok_fail_on_execute() {
    let (mut suite, mut accounts, replier_addr) = setup();

    suite
        .execute(
            &mut accounts["owner"],
            replier_addr,
            &ExecuteMsg::perform(
                Some(btree_map!(1 => "one".to_string())),
                ExecuteMsg::fail("reply_always_ok_fail_on_execute"),
                ReplyOn::always(&ReplyMsg::Fail(ExecuteMsg::ok(Some(
                    btree_map!(3 => "three".to_string()),
                ))))
                .unwrap(),
            ),
            Coins::default(),
        )
        .unwrap();

    let res: MapData = suite.query_wasm(replier_addr, &Empty {}).unwrap();

    assert_eq!(
        res,
        btree_map!(1 => "one".to_string(), 3 => "three".to_string())
    );
}

#[test]
fn reply_always_ok_fail_on_reply() {
    let (mut suite, mut accounts, replier_addr) = setup();

    let res = suite.execute(
        &mut accounts["owner"],
        replier_addr,
        &ExecuteMsg::perform(
            Some(btree_map!(1 => "one".to_string())),
            ExecuteMsg::ok(Some(btree_map!(2 => "two".to_string()))),
            ReplyOn::always(&ReplyMsg::Ok(ExecuteMsg::fail(
                "reply_always_ok_fail_on_execute",
            )))
            .unwrap(),
        ),
        Coins::default(),
    );

    println!("{:?}", res)
}
