use {
    assertor::*,
    dango_testing::setup_test_with_indexer,
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
    },
    grug::{Addressable, Coins, Message, NonEmpty, ResultExt},
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
};

#[test]
fn index_transfer_events() {
    let (mut suite, mut accounts, _, contracts) = setup_test_with_indexer();

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
        },
        Coins::one("uusdc", 100_000_000).unwrap(),
    )
    .unwrap()];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish();

    // The 2 transfers should have been indexed.
    suite
        .app
        .indexer
        .handle
        .block_on(async {
            let blocks = indexer_sql::entity::blocks::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(blocks).has_length(1);

            let transfers = dango_indexer_sql::entity::transfers::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(transfers).has_length(2);

            assert_that!(transfers
                .iter()
                .map(|t| t.amount.as_str())
                .collect::<Vec<_>>())
            .is_equal_to(vec!["100000000", "100000000"]);

            Ok::<_, anyhow::Error>(())
        })
        .expect("Can't fetch transfers");

    let msg =
        Message::transfer(accounts.user1.address(), Coins::one("uusdc", 123).unwrap()).unwrap();

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(vec![msg]),
        )
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    // The transfer should have been indexed.
    suite
        .app
        .indexer
        .handle
        .block_on(async {
            let blocks = indexer_sql::entity::blocks::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(blocks).has_length(2);

            let transfers = dango_indexer_sql::entity::transfers::Entity::find()
                .filter(dango_indexer_sql::entity::transfers::Column::BlockHeight.eq(2))
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(transfers).has_length(1);

            assert_that!(transfers
                .iter()
                .map(|t| t.amount.as_str())
                .collect::<Vec<_>>())
            .is_equal_to(vec!["123"]);

            Ok::<_, anyhow::Error>(())
        })
        .expect("Can't fetch transfers");
}
