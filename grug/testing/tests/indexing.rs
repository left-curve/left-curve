use {
    assertor::*,
    grug_app::Indexer,
    grug_testing::TestBuilder,
    grug_types::{Coins, Denom, Message, ResultExt},
    indexer_sql::entity,
    sea_orm::EntityTrait,
    std::str::FromStr,
};

#[test]
fn index_block_with_nonblocking_indexer() {
    let denom = Denom::from_str("ugrug").unwrap();

    let mut indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .expect("Can't create indexer");

    indexer.start().expect("Can't start indexer");

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer.clone())
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

    // Force the runtime to wait for the async indexer to finish
    indexer.shutdown().expect("Can't shutdown indexer");

    // ensure block was saved
    indexer
        .runtime
        .clone()
        .expect("Can't get runtime")
        .block_on(async {
            let block = entity::blocks::Entity::find()
                .one(&indexer.context.db)
                .await
                .expect("Can't fetch blocks");
            assert_that!(block).is_some();
            assert_that!(block.unwrap().block_height).is_equal_to(1);

            let transactions = entity::transactions::Entity::find()
                .all(&indexer.context.db)
                .await
                .expect("Can't fetch transactions");
            assert_that!(transactions).is_not_empty();

            let messages = entity::messages::Entity::find()
                .all(&indexer.context.db)
                .await
                .expect("Can't fetch messages");
            assert_that!(messages).is_not_empty();

            let events = entity::events::Entity::find()
                .all(&indexer.context.db)
                .await
                .expect("Can't fetch events");
            assert_that!(events).is_not_empty();
        });
}
