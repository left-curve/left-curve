use {
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{
        HyperlaneTestSuite, add_user_public_key, create_user_and_account, setup_test_with_indexer,
    },
    grug_app::Indexer,
    sea_orm::EntityTrait,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_single_user_multiple_public_keys() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_context) =
        setup_test_with_indexer().await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut test_account1 =
        create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    let (pk, key_hash) = add_user_public_key(&mut suite, &contracts, &mut test_account1);

    suite.app.indexer.wait_for_finish();

    let users_and_public_keys: Vec<(entity::users::Model, Vec<entity::public_keys::Model>)> =
        dango_indexer_sql::entity::users::Entity::find()
            .find_with_related(dango_indexer_sql::entity::public_keys::Entity)
            .all(&dango_context.db)
            .await?;

    assert_that!(users_and_public_keys).has_length(1);
    assert_that!(users_and_public_keys[0].1).has_length(2);

    let key_hashes = users_and_public_keys[0]
        .1
        .iter()
        .map(|t| &t.key_hash)
        .collect::<Vec<_>>();

    let public_keys = users_and_public_keys[0]
        .1
        .iter()
        .map(|t| &t.public_key)
        .collect::<Vec<_>>();

    assert_that!(key_hashes).contains(&key_hash.to_string());
    assert_that!(key_hashes).contains(&test_account1.first_key_hash().to_string());

    assert_that!(public_keys).contains(&pk.to_string());
    assert_that!(public_keys).contains(&test_account1.first_key().to_string());

    Ok(())
}
