use {
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::create_accounts,
    grug::{Inner, setup_tracing_subscriber},
    sea_orm::EntityTrait,
};

#[test]
fn index_account_creations() {
    setup_tracing_subscriber(tracing::Level::INFO);
    let (suite, test_account, _) = create_accounts();

    suite
        .app
        .indexer
        .handle
        .block_on(async {
            let users = dango_indexer_sql::entity::users::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            let accounts = dango_indexer_sql::entity::accounts::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            let account_users = dango_indexer_sql::entity::accounts_users::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            let public_keys = dango_indexer_sql::entity::public_keys::Entity::find()
                .all(&suite.app.indexer.context.db)
                .await?;

            assert_that!(
                users
                    .iter()
                    .map(|t| t.username.as_str())
                    .collect::<Vec<_>>()
            )
            .is_equal_to(vec![test_account.username.as_ref()]);

            assert_that!(users).has_length(1);
            assert_that!(accounts).has_length(1);
            assert_that!(account_users).has_length(1);
            assert_that!(public_keys).has_length(1);

            let public_key = public_keys.first().unwrap();

            assert_that!(&public_key.username).is_equal_to(test_account.username.inner());
            assert_that!(public_key.key_hash)
                .is_equal_to(test_account.first_key_hash().to_string());
            assert_that!(public_key.public_key).is_equal_to(test_account.first_key().to_string());

            // dbg!(users);
            // dbg!(public_keys);
            // dbg!(accounts);
            // dbg!(account_users);

            Ok::<_, anyhow::Error>(())
        })
        .expect("Can't fetch accounts");
}

#[test]
fn index_previous_blocks() {
    setup_tracing_subscriber(tracing::Level::INFO);
    let (suite, test_account, _) = create_accounts();

    suite
        .app
        .indexer
        .handle
        .block_on(async {
            let accounts: Vec<(entity::accounts::Model, Vec<entity::users::Model>)> =
                dango_indexer_sql::entity::accounts::Entity::find()
                    .find_with_related(dango_indexer_sql::entity::users::Entity)
                    .all(&suite.app.indexer.context.db)
                    .await?;

            assert_that!(accounts).has_length(1);

            assert_that!(
                accounts[0]
                    .1
                    .iter()
                    .map(|t| t.username.as_str())
                    .collect::<Vec<_>>()
            )
            .is_equal_to(vec![test_account.username.as_ref()]);

            Ok::<_, anyhow::Error>(())
        })
        .expect("Can't fetch accounts");
}
