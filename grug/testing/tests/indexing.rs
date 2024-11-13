use {
    assertor::*,
    grug_testing::TestBuilder,
    grug_types::{Coins, Denom, Message},
    sea_orm::EntityTrait,
    sea_orm::QueryOrder,
    std::str::FromStr,
};

#[test]
fn index_block() {
    let denom = Denom::from_str("ugrug").unwrap();
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["sender"].address;

    let _outcome = suite.send_message_with_gas(
        &mut accounts["sender"],
        2000,
        Message::transfer(to, Coins::new()).unwrap(),
    );

    // ensure block was saved
    suite
        .app
        .indexer_app
        .runtime
        .block_on(async {
            let block = indexer_entity::blocks::Entity::find()
                .one(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch blocks");
            assert_that!(block).is_some();
            dbg!(&block);
            assert_that!(block.unwrap().block_height).is_equal_to(1);

            let transaction = indexer_entity::transactions::Entity::find()
                .one(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch transactions");
            dbg!(&transaction);

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
            let block = indexer_entity::blocks::Entity::find()
                .order_by_desc(indexer_entity::blocks::Column::BlockHeight)
                .one(&suite.app.indexer_app.context.db)
                .await
                .expect("Can't fetch blocks");
            assert_that!(block).is_some();
            dbg!(&block);
            assert_that!(block.unwrap().block_height).is_equal_to(2);
            Ok::<(), sea_orm::DbErr>(())
        })
        .expect("Can't commit txn");
}
