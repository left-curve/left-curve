use {
    crate::sql::replier,
    assertor::*,
    dango_app::{Db, Indexer},
    dango_indexer_cache::cache_file::CacheFile,
    dango_indexer_sql::entity,
    dango_primitives::{
        Addressable, Block, BlockInfo, BlockOutcome, Coins, Empty, Hash, Message, ReplyOn,
        ResultExt,
    },
    dango_testing::{TestOption, setup_test_naive_with_indexer},
    dango_types::constants::usdc,
    dango_vm_rust::ContractBuilder,
    replier::{ExecuteMsg, ReplyMsg},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder},
};

#[tokio::test(flavor = "multi_thread")]
async fn index_block() {
    let (mut suite, mut accounts, _, _, _, httpd_context, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default().with_mocked_clickhouse()).await;

    let to = accounts.owner.address();

    suite
        .send_message_with_gas(
            &mut accounts.user1,
            1_000_000,
            Message::transfer(to, Coins::one(usdc::DENOM.clone(), 100).unwrap()).unwrap(),
        )
        .await
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    // ensure block was saved
    let block = entity::blocks::Entity::find()
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();
    assert_that!(block.unwrap().block_height).is_equal_to(1);

    let transactions = entity::transactions::Entity::find()
        .all(&httpd_context.db)
        .await
        .expect("Can't fetch transactions");
    assert_that!(transactions).is_not_empty();

    let messages = entity::messages::Entity::find()
        .all(&httpd_context.db)
        .await
        .expect("Can't fetch messages");

    assert_that!(messages).is_not_empty();

    let events = entity::events::Entity::find()
        .all(&httpd_context.db)
        .await
        .expect("Can't fetch events");

    // Verify message_id is set correctly based on message_idx
    for event in events.iter() {
        assert_that!(event.message_id.is_some()).is_equal_to(event.message_idx.is_some());
    }

    assert_that!(events).is_not_empty();
}

/// This test is to ensure the indexer will index previous block not yet indexed.
/// This happens if the process crash after the block was saved on disk, and
/// before it was indexed.
#[tokio::test(flavor = "multi_thread")]
async fn parse_previous_block_after_restart() {
    let (mut suite, mut accounts, _, _, _, httpd_context, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default().with_mocked_clickhouse()).await;

    let to = accounts.owner.address();

    suite
        .send_message_with_gas(
            &mut accounts.user1,
            1_000_000,
            Message::transfer(to, Coins::one(usdc::DENOM.clone(), 100).unwrap()).unwrap(),
        )
        .await
        .should_succeed();

    suite
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    // Force the runtime to shutdown or when reusing this `start` would fail
    suite
        .app
        .indexer
        .shutdown()
        .await
        .expect("Can't shutdown indexer");

    tracing::warn!("Shut down indexer");

    // 1. Delete database block height 1
    entity::blocks::Entity::delete_block_and_data(&httpd_context.db, 1)
        .await
        .unwrap();

    // 1 bis. Verify the block height 1 is deleted
    let block = entity::blocks::Entity::find()
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_none();

    // 2. on disk block already exists (we keep blocks)

    // 3. Start the indexer
    suite
        .app
        .indexer
        .start(&suite.app.db.state_storage(None).expect("Can't get storage"))
        .await
        .expect("Can't start indexer");

    tracing::warn!("Start indexer");

    // 4. Verify the block height 1 is indexed
    let block = entity::blocks::Entity::find()
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();
    assert_that!(block.unwrap().block_height).is_equal_to(1);

    // 5. Send a transaction
    suite
        .send_message_with_gas(
            &mut accounts.user1,
            1_000_000,
            Message::transfer(to, Coins::one(usdc::DENOM.clone(), 100).unwrap()).unwrap(),
        )
        .await
        .should_succeed();

    // 5 bis. Force the runtime to wait for the async indexer task to finish
    suite
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    // 6. Verify the block height 2 is indexed
    let block = entity::blocks::Entity::find()
        .order_by_desc(entity::blocks::Column::BlockHeight)
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();
    assert_that!(block.unwrap().block_height).is_equal_to(2);
}

/// This test is to ensure the indexer will reindex previous block already indexed.
/// This happens if the process crash after the block was saved on disk,
/// after it was indexed, and before the tmp file was deleted.
#[tokio::test(flavor = "multi_thread")]
async fn no_sql_index_error_after_restart() {
    let (mut suite, mut accounts, _, _, _, httpd_context, cache_context, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default().with_mocked_clickhouse()).await;

    let to = accounts.owner.address();

    suite
        .send_message_with_gas(
            &mut accounts.user1,
            1_000_000,
            Message::transfer(to, Coins::one(usdc::DENOM.clone(), 100).unwrap()).unwrap(),
        )
        .await
        .should_succeed();

    suite
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    // Force the runtime to shutdown or when reusing this `start` would fail
    suite
        .app
        .indexer
        .shutdown()
        .await
        .expect("Can't shutdown indexer");

    // 1. Verify the block height 1 is indexed
    let block = entity::blocks::Entity::find()
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();

    // 2. Manually create a block in cache with block height 1
    let block_info = BlockInfo {
        height: 1,
        timestamp: Default::default(),
        hash: Hash::ZERO,
    };
    let block_outcome = BlockOutcome {
        height: 1,
        app_hash: Hash::ZERO,
        cron_outcomes: vec![],
        tx_outcomes: vec![],
    };
    let block = Block {
        info: block_info,
        txs: vec![],
    };

    let block_filename = cache_context.indexer_path.block_path(block_info.height);
    let block_to_index = CacheFile::new(block_filename, block, block_outcome);

    block_to_index
        .save_to_disk()
        .expect("Can't save block on disk");

    tracing::info!("Starting indexer again");

    // 3. Start the indexer
    suite
        .app
        .indexer
        .start(&suite.app.db.state_storage(None).expect("Can't get storage"))
        .await
        .expect("Can't start indexer");

    // 4. Verify the block height 1 is still indexed
    let block = entity::blocks::Entity::find()
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();
    assert_that!(block.unwrap().block_height).is_equal_to(1);

    // 5. Send a transaction
    suite
        .send_message_with_gas(
            &mut accounts.user1,
            1_000_000,
            Message::transfer(to, Coins::one(usdc::DENOM.clone(), 100).unwrap()).unwrap(),
        )
        .await
        .should_succeed();

    // 5 bis. Force the runtime to wait for the async indexer task to finish
    suite
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    // 6. Verify the block height 2 is indexed
    let block = entity::blocks::Entity::find()
        .order_by_desc(entity::blocks::Column::BlockHeight)
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();

    let block = block.unwrap();
    assert_that!(block.block_height).is_equal_to(2);

    assert!(!block.app_hash.is_empty());
}

/// Ensure that flatten events are indexed correctly.
#[tokio::test(flavor = "multi_thread")]
async fn index_block_events() {
    let (mut suite, mut accounts, _, _, _, httpd_context, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default().with_mocked_clickhouse()).await;

    let replier_code = ContractBuilder::new(Box::new(replier::instantiate))
        .with_execute(Box::new(replier::execute))
        .with_query(Box::new(replier::query))
        .with_reply(Box::new(replier::reply))
        .build();

    let replier_addr = suite
        .upload_and_instantiate(
            &mut accounts.owner,
            replier_code,
            &Empty {},
            "salt",
            Some("label"),
            None,
            Coins::default(),
        )
        .await
        .should_succeed()
        .address;

    let msg = ExecuteMsg::perform(
        "1",
        ExecuteMsg::ok("2"),
        ReplyOn::success(&ReplyMsg::Ok(ExecuteMsg::ok("1.1"))).unwrap(),
    );

    suite
        .execute(&mut accounts.owner, replier_addr, &msg, Coins::default())
        .await
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    // ensure block was saved
    let block = entity::blocks::Entity::find()
        .one(&httpd_context.db)
        .await
        .expect("Can't fetch blocks");
    assert_that!(block).is_some();
    assert_that!(block.unwrap().block_height).is_equal_to(1);

    let transactions = entity::transactions::Entity::find()
        .all(&httpd_context.db)
        .await
        .expect("Can't fetch transactions");
    assert_that!(transactions).is_not_empty();

    let messages = entity::messages::Entity::find()
        .all(&httpd_context.db)
        .await
        .expect("Can't fetch messages");
    assert_that!(messages).is_not_empty();

    let events = entity::events::Entity::find()
        .filter(entity::events::Column::BlockHeight.eq(2))
        .all(&httpd_context.db)
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
        .all(&httpd_context.db)
        .await
        .expect("Can't fetch events");
    assert_that!(events).is_not_empty();
}
