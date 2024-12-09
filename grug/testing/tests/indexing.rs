use {
    assertor::*,
    grug_app::{Db, Indexer},
    grug_testing::TestBuilder,
    grug_types::{BlockInfo, Coins, Denom, Hash, Message, ResultExt},
    indexer_sql::{block::BlockToIndex, entity},
    sea_orm::{EntityTrait, QueryOrder},
    std::str::FromStr,
};

#[test]
fn index_block_with_nonblocking_indexer() {
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

    assert_that!(suite.app.indexer().indexing).is_true();

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
        )
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer().wait_for_finish();

    // ensure block was saved
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer().context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(1);

        let transactions = entity::transactions::Entity::find()
            .all(&suite.app.indexer().context.db)
            .await
            .expect("Can't fetch transactions");
        assert_that!(transactions).is_not_empty();

        let messages = entity::messages::Entity::find()
            .all(&suite.app.indexer().context.db)
            .await
            .expect("Can't fetch messages");
        assert_that!(messages).is_not_empty();

        let events = entity::events::Entity::find()
            .all(&suite.app.indexer().context.db)
            .await
            .expect("Can't fetch events");
        assert_that!(events).is_not_empty();
    });
}

/// This test is to ensure the indexer will index previous block not yet indexed.
/// This happens if the process crash after the block was saved on disk, and
/// before it was indexed.
#[test]
fn parse_previous_block_after_restart() {
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
    suite
        .app
        .indexer
        .delete_block_from_db(1)
        .expect("Can't delete block");

    // 1 bis. Verify the block height 1 is deleted
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer().context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_none();
    });

    // 2. Manually create a block in cache with block height 1
    let block_info = BlockInfo {
        height: 1,
        timestamp: Default::default(),
        hash: Hash::ZERO,
    };
    let block_to_index = BlockToIndex::new(
        block_info,
        suite
            .app
            .indexer
            .block_tmp_filename(block_info.height)
            .to_string_lossy()
            .to_string(),
    );
    block_to_index
        .save_tmp_file()
        .expect("Can't save tmp_file block");

    // 3. Start the indexer
    suite
        .app
        .indexer
        .start(&suite.app.db.state_storage(None).expect("Can't get storage"))
        .expect("Can't start indexer");

    // 4. Verify the block height 1 is indexed
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer().context.db)
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
    suite.app.indexer().wait_for_finish();

    // 6. Verify the block height 2 is indexed
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&suite.app.indexer().context.db)
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
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer().context.db)
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
    let block_to_index = BlockToIndex::new(
        block_info,
        suite
            .app
            .indexer
            .block_tmp_filename(block_info.height)
            .to_string_lossy()
            .to_string(),
    );
    block_to_index
        .save_tmp_file()
        .expect("Can't save tmp_file block");

    // 3. Start the indexer
    suite
        .app
        .indexer
        .start(&suite.app.db.state_storage(None).expect("Can't get storage"))
        .expect("Can't start indexer");

    // 4. Verify the block height 1 is still indexed
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .one(&suite.app.indexer().context.db)
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
    suite.app.indexer().wait_for_finish();

    // 6. Verify the block height 2 is indexed
    suite.app.indexer().handle.block_on(async {
        let block = entity::blocks::Entity::find()
            .order_by_desc(entity::blocks::Column::BlockHeight)
            .one(&suite.app.indexer().context.db)
            .await
            .expect("Can't fetch blocks");
        assert_that!(block).is_some();
        assert_that!(block.unwrap().block_height).is_equal_to(2);
    });
}
