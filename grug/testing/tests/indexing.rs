use {
    assertor::*,
    grug_testing::TestBuilder,
    grug_types::{Coins, Denom, Message},
    indexer_core::IndexerTrait,
    indexer_sql::entity,
    sea_orm::{EntityTrait, QueryOrder},
    std::str::FromStr,
};

#[test]
fn index_block_with_blocking_indexer() {
    let denom = Denom::from_str("ugrug").unwrap();
    let mut indexer = indexer_sql::blocking_indexer::Indexer::new_with_database("sqlite::memory:")
        .expect("can't create indexer");
    indexer.start().expect("Can't start indexer");
    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;
    // let from = accounts["owner"].address;

    // dbg!(&accounts);

    let _outcome = suite.send_message_with_gas(
        &mut accounts["sender"],
        2000,
        Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
    );

    // ensure block was saved
    suite
        .app
        .indexer_app
        .runtime
        .block_on(async {
            let block = entity::blocks::Entity::find()
                .one(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch blocks");
            assert_that!(block).is_some();
            // dbg!(&block);
            assert_that!(block.unwrap().block_height).is_equal_to(1);

            let transactions = entity::transactions::Entity::find()
                .all(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch transactions");
            assert_that!(transactions).is_not_empty();
            // dbg!(&transactions);

            let messages = entity::messages::Entity::find()
                .all(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch messages");
            assert_that!(messages).is_not_empty();
            // dbg!(&messages);

            let events = entity::events::Entity::find()
                .all(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch events");
            assert_that!(events).is_not_empty();
            // dbg!(&events);

            Ok::<(), sea_orm::DbErr>(())
        })
        .expect("Can't commit txn");

    let _outcome = suite.send_message_with_gas(
        &mut accounts["sender"],
        3000,
        Message::transfer(to, Coins::new()).unwrap(),
    );

    // ensure block was saved
    suite
        .app
        .indexer_app
        .runtime
        .block_on(async {
            let block = entity::blocks::Entity::find()
                .order_by_desc(entity::blocks::Column::BlockHeight)
                .one(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch blocks");
            assert_that!(block).is_some();
            // dbg!(&block);
            assert_that!(block.unwrap().block_height).is_equal_to(2);
            Ok::<(), sea_orm::DbErr>(())
        })
        .expect("Can't commit txn");
}

#[test]
fn index_block_with_nonblocking_indexer() {
    let denom = Denom::from_str("ugrug").unwrap();
    let mut indexer = indexer_sql::blocking_indexer::Indexer::new_with_database("sqlite::memory:")
        .expect("can't create indexer");
    indexer.start().expect("Can't start indexer");

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;
    // let from = accounts["owner"].address;

    // dbg!(&accounts);

    let _outcome = suite.send_message_with_gas(
        &mut accounts["sender"],
        2000,
        Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
    );

    // Force the runtime to wait for the async indexer to finish
    suite
        .app
        .indexer_app
        .shutdown()
        .expect("Can't shutdown indexer");

    // ensure block was saved
    suite
        .app
        .indexer_app
        .runtime
        .block_on(async {
            let block = entity::blocks::Entity::find()
                .one(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch blocks");
            assert_that!(block).is_some();
            // dbg!(&block);
            assert_that!(block.unwrap().block_height).is_equal_to(1);

            let transactions = entity::transactions::Entity::find()
                .all(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch transactions");
            assert_that!(transactions).is_not_empty();
            // dbg!(&transactions);

            let messages = entity::messages::Entity::find()
                .all(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch messages");
            assert_that!(messages).is_not_empty();
            // dbg!(&messages);

            let events = entity::events::Entity::find()
                .all(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch events");
            assert_that!(events).is_not_empty();
            // dbg!(&events);

            Ok::<(), sea_orm::DbErr>(())
        })
        .expect("Can't commit txn");
}
