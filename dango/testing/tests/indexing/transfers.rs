use {
    assertor::*,
    dango_testing::setup_test_with_indexer,
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
        constants::usdc,
    },
    grug::{Addressable, Coins, Message, NonEmpty, ResultExt},
    grug_app::Indexer,
    sea_orm::{ColumnTrait, EntityTrait, QueryFilter},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_transfer_events() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, ..) = setup_test_with_indexer();

    // Copied from benchmarks.rs
    let msgs = vec![
        Message::execute(
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {
                params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )
        .unwrap(),
    ];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish();

    {
        let sql_context = suite
            .app
            .indexer
            .context()
            .data()
            .lock()
            .unwrap()
            .get::<indexer_sql::Context>()
            .expect("SQL context should be stored")
            .clone();

        // The 2 transfers should have been indexed.

        let blocks = indexer_sql::entity::blocks::Entity::find()
            .all(&sql_context.db)
            .await?;

        assert_that!(blocks).has_length(1);

        let transfers = dango_indexer_sql::entity::transfers::Entity::find()
            .all(&sql_context.db)
            .await?;

        assert_that!(transfers).has_length(2);

        assert_that!(
            transfers
                .iter()
                .map(|t| t.amount.as_str())
                .collect::<Vec<_>>()
        )
        .is_equal_to(vec!["100000000", "100000000"]);
    }

    let msg = Message::transfer(
        accounts.user1.address(),
        Coins::one(usdc::DENOM.clone(), 123).unwrap(),
    )
    .unwrap();

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(vec![msg]),
        )
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    let sql_context = suite
        .app
        .indexer
        .context()
        .data()
        .lock()
        .unwrap()
        .get::<indexer_sql::Context>()
        .expect("SQL context should be stored")
        .clone();

    // The transfer should have been indexed.
    let blocks = indexer_sql::entity::blocks::Entity::find()
        .all(&sql_context.db)
        .await?;

    assert_that!(blocks).has_length(2);

    let transfers = dango_indexer_sql::entity::transfers::Entity::find()
        .filter(dango_indexer_sql::entity::transfers::Column::BlockHeight.eq(2))
        .all(&sql_context.db)
        .await?;

    assert_that!(transfers).has_length(1);

    assert_that!(
        transfers
            .iter()
            .map(|t| t.amount.as_str())
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec!["123"]);

    Ok(())
}
