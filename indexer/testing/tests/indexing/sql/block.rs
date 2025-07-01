use {
    assertor::*,
    grug_app::{Db, Indexer},
    grug_testing::TestBuilder,
    grug_types::{
        Block, BlockInfo, BlockOutcome, Coin, Coins, Denom, Empty, Hash, Message, ReplyOn,
        ResultExt,
    },
    grug_vm_rust::ContractBuilder,
    indexer_sql::{block_to_index::BlockToIndex, entity},
    replier::{ExecuteMsg, ReplyMsg},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
    std::str::FromStr,
};

#[test]
fn index_block() {
    let denom = Denom::from_str("ugrug").unwrap();

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .expect("Can't create indexer");

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;

    assert_that!(suite.app.indexer.indexing).is_true();

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
        )
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // ensure block was saved
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(1);

        let transactions = entity::transactions::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch transactions");
        assert_that!(transactions).is_not_empty();

        let messages = entity::messages::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch messages");

        assert_that!(messages).is_not_empty();

        let events = entity::events::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch events");

        // Verify message_id is set correctly based on message_idx
        for event in events.iter() {
            assert_that!(event.message_id.is_some()).is_equal_to(event.message_idx.is_some());
        }

        assert_that!(events).is_not_empty();
    });
}

/// This test is to ensure the indexer will index previous block not yet indexed.
/// This happens if the process crash after the block was saved on disk, and
/// before it was indexed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parse_previous_block_after_restart() {
    let denom = Denom::from_str("ugrug").unwrap();

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_keep_blocks(true)
        .with_memory_database()
        .build()
        .expect("Can't create indexer");

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
        )
        .should_succeed();

    // Force the runtime to shutdown or when reusing this `start` would fail
    suite
        .app
        .indexer
        .shutdown()
        .expect("Can't shutdown indexer");

    // 1. Delete database block height 1
    entity::blocks::Entity::delete_block_and_data(&suite.app.indexer.context.db, 1)
        .await
        .unwrap();

    // 1 bis. Verify the block height 1 is deleted
    let block = entity::blocks::Entity::find()
        .one(&suite.app.indexer.context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_none();

    // 2. on disk block already exists (we keep blocks)

    // 3. Start the indexer
    suite
        .app
        .indexer
        .start(&suite.app.db.state_storage(None).expect("Can't get storage"))
        .expect("Can't start indexer");

    // 4. Verify the block height 1 is indexed
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(1);
    });

    // 5. Send a transaction
    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
        )
        .should_succeed();

    // 5 bis. Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // 6. Verify the block height 2 is indexed
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(2);
    });
}

/// This test is to ensure the indexer will reindex previous block already indexed.
/// This happens if the process crash after the block was saved on disk,
/// after it was indexed, and before the tmp file was deleted.
#[test]
fn no_sql_index_error_after_restart() {
    let denom = Denom::from_str("ugrug").unwrap();

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .expect("Can't create indexer");

    let indexer_path = indexer.indexer_path.clone();

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
        )
        .should_succeed();

    // Force the runtime to shutdown or when reusing this `start` would fail
    suite
        .app
        .indexer
        .shutdown()
        .expect("Can't shutdown indexer");

    // 1. Verify the block height 1 is indexed
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
    });

    // 2. Manually create a block in cache with block height 1
    let block_info = BlockInfo {
        height: 1,
        timestamp: Default::default(),
        hash: Hash::ZERO,
    };
    let block_outcome = BlockOutcome {
        app_hash: Hash::ZERO,
        cron_outcomes: vec![],
        tx_outcomes: vec![],
    };
    let block = Block {
        info: block_info,
        txs: vec![],
    };

    let block_filename = indexer_path.block_path(block_info.height);
    let block_to_index = BlockToIndex::new(block_filename, block, block_outcome);

    block_to_index
        .save_to_disk()
        .expect("Can't save block on disk");

    // 3. Start the indexer
    suite
        .app
        .indexer
        .start(&suite.app.db.state_storage(None).expect("Can't get storage"))
        .expect("Can't start indexer");

    // 4. Verify the block height 1 is still indexed
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(1);
    });

    // 5. Send a transaction
    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
        )
        .should_succeed();

    // 5 bis. Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // 6. Verify the block height 2 is indexed
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();

        let block = block.unwrap();
        assert_that!(block.block_height).is_equal_to(2);

        assert!(!block.app_hash.is_empty());
    });
}

pub mod replier {
    use {
        grug_storage::Set,
        grug_types::{
            Coins, Empty, GenericResult, ImmutableCtx, Json, JsonSerExt, Message, MutableCtx,
            Order, QueryRequest, ReplyOn, Response, StdError, StdResult, SubMessage, SubMsgResult,
            SudoCtx,
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

    pub const DEPTHS: Set<String> = Set::new("s");

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
                DEPTHS.insert(ctx.storage, deep)?;

                Ok(Response::new())
            },
            ExecuteMsg::Perform {
                deep,
                next,
                reply_on,
            } => {
                DEPTHS.insert(ctx.storage, deep)?;

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
                let res = DEPTHS
                    .range(ctx.storage, None, None, Order::Ascending)
                    .collect::<StdResult<Vec<_>>>()?;
                res.to_json_value()
            },
        }
    }

    pub fn reply(ctx: SudoCtx, msg: ReplyMsg, res: SubMsgResult) -> StdResult<Response> {
        let msg = match (res, msg) {
            (GenericResult::Err(_), ReplyMsg::Fail(execute_msg))
            | (GenericResult::Ok(_), ReplyMsg::Ok(execute_msg)) => execute_msg,
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

/// Ensure that flatten events are indexed correctly.
#[test]
fn index_block_events() {
    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .expect("Can't create indexer");

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
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

    let msg = ExecuteMsg::perform(
        "1",
        ExecuteMsg::ok("2"),
        ReplyOn::success(&ReplyMsg::Ok(ExecuteMsg::ok("1.1"))).unwrap(),
    );

    suite
        .execute(&mut accounts["owner"], replier_addr, &msg, Coins::default())
        .should_succeed();

    assert_that!(suite.app.indexer.indexing).is_true();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // ensure block was saved
    suite.app.indexer.handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(1);

        let transactions = entity::transactions::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch transactions");
        assert_that!(transactions).is_not_empty();

        let messages = entity::messages::Entity::find()
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch messages");
        assert_that!(messages).is_not_empty();

        let events = entity::events::Entity::find()
            .filter(entity::events::Column::BlockHeight.eq(2))
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch events");
        assert_that!(events).is_not_empty();

        // Check for gaps
        let event_idxs = events.iter().map(|e| e.event_idx).collect::<Vec<_>>();
        let min_idx = event_idxs[0];
        let max_idx = event_idxs[event_idxs.len() - 1];
        assert_that!(event_idxs.len() as i32).is_equal_to(max_idx - min_idx + 1);

        // check for parent events
        let events = entity::events::Entity::find()
            .filter(entity::events::Column::ParentId.is_not_null())
            .all(&suite.app.indexer.context.db)
            .await
            .expect("Can't fetch events");
        assert_that!(events).is_not_empty();
    });
}

/// Ensure the indexed blocks are compressed on disk.
#[test]
fn blocks_on_disk_compressed() {
    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_keep_blocks(true)
        .build()
        .expect("Can't create indexer");

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coin::new("usdc", 100_000).unwrap())
        .add_account("sender", Coins::new())
        .set_owner("owner")
        .build();

    // Just create a block.
    let replier_code = ContractBuilder::new(Box::new(replier::instantiate))
        .with_execute(Box::new(replier::execute))
        .with_query(Box::new(replier::query))
        .with_reply(Box::new(replier::reply))
        .build();

    let _ = suite
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

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    let mut block_path = suite.app.indexer.indexer_path.block_path(1);

    block_path.set_extension("borsh.xz");

    assert!(
        block_path.exists(),
        "Compressed block_file should exist: {}",
        block_path.display()
    );

    block_path.set_extension("");

    assert!(
        !block_path.exists(),
        "Decompressed block_file should not exist: {}",
        block_path.display()
    );
}
