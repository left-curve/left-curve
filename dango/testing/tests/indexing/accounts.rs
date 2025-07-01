use {
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{
        HyperlaneTestSuite, add_account_with_existing_user, create_user_and_account,
        setup_test_with_indexer,
    },
    grug::Inner,
    grug_app::Indexer,
    itertools::Itertools,
    sea_orm::EntityTrait,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_account_creations() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _) = setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish();

    let sql_context = suite
        .app
        .indexer
        .context()
        .data()
        .get(&indexer_sql::ContextKey)
        .expect("SQL context should be stored")
        .value()
        .clone();

    let users = dango_indexer_sql::entity::users::Entity::find()
        .all(&sql_context.db)
        .await?;

    let accounts = dango_indexer_sql::entity::accounts::Entity::find()
        .all(&sql_context.db)
        .await?;

    let account_users = dango_indexer_sql::entity::accounts_users::Entity::find()
        .all(&sql_context.db)
        .await?;

    let public_keys = dango_indexer_sql::entity::public_keys::Entity::find()
        .all(&sql_context.db)
        .await?;

    assert_that!(
        users
            .iter()
            .map(|t| t.username.as_str())
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec![user.username.as_ref()]);

    assert_that!(users).has_length(1);
    assert_that!(accounts).has_length(1);
    assert_that!(account_users).has_length(1);
    assert_that!(public_keys).has_length(1);

    let public_key = public_keys.first().unwrap();

    assert_that!(&public_key.username).is_equal_to(user.username.inner());
    assert_that!(public_key.key_hash).is_equal_to(user.first_key_hash().to_string());
    assert_that!(public_key.public_key).is_equal_to(user.first_key().to_string());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_previous_blocks() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _) = setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish();

    let sql_context = suite
        .app
        .indexer
        .context()
        .data()
        .get(&indexer_sql::ContextKey)
        .expect("SQL context should be stored")
        .value()
        .clone();

    let accounts: Vec<(entity::accounts::Model, Vec<entity::users::Model>)> =
        dango_indexer_sql::entity::accounts::Entity::find()
            .find_with_related(dango_indexer_sql::entity::users::Entity)
            .all(&sql_context.db)
            .await?;

    assert_that!(accounts).has_length(1);

    assert_that!(
        accounts[0]
            .1
            .iter()
            .map(|t| t.username.as_str())
            .collect::<Vec<_>>()
    )
    .is_equal_to(vec![user.username.as_ref()]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_single_user_multiple_spot_accounts() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _) = setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut test_account1 =
        create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    let test_account2 = add_account_with_existing_user(&mut suite, &contracts, &mut test_account1);

    suite.app.indexer.wait_for_finish();

    let sql_context = suite
        .app
        .indexer
        .context()
        .data()
        .get(&indexer_sql::ContextKey)
        .expect("SQL context should be stored")
        .value()
        .clone();

    let accounts: Vec<(entity::accounts::Model, Vec<entity::users::Model>)> =
        dango_indexer_sql::entity::accounts::Entity::find()
            .find_with_related(dango_indexer_sql::entity::users::Entity)
            .all(&sql_context.db)
            .await?;

    assert_that!(accounts).has_length(2);

    let usernames = accounts
        .iter()
        .map(|(_, users)| &users[0].username)
        .unique()
        .collect::<Vec<_>>();

    let addresses = accounts
        .iter()
        .map(|(account, _)| &account.address)
        .unique()
        .collect::<Vec<_>>();

    assert_that!(usernames).has_length(1);
    assert_that!(usernames[0].as_str()).is_equal_to("user");

    assert_that!(addresses).has_length(2);
    assert_that!(addresses).contains(&test_account1.address.into_inner().to_string());
    assert_that!(addresses).contains(&test_account2.address.into_inner().to_string());

    Ok(())
}
