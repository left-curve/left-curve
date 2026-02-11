use {
    super::call_graphql_query,
    assertor::*,
    dango_testing::{
        HyperlaneTestSuite, TestOption, add_user_public_key, create_user_and_account,
        setup_test_with_indexer,
    },
    graphql_client::GraphQLQuery,
    grug_app::Indexer,
    indexer_client::{User, Users, user, users},
};

#[tokio::test(flavor = "multi_thread")]
async fn query_user() -> anyhow::Result<()> {
    let (
        suite,
        mut accounts,
        codes,
        contracts,
        validator_sets,
        _,
        dango_httpd_context,
        _,
        _db_guard,
    ) = setup_test_with_indexer(TestOption::default()).await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, users::ResponseData>(
                    dango_httpd_context,
                    Users::build_query(users::Variables::default()),
                )
                .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.users.nodes).is_not_empty();
                let first_user = &data.users.nodes[0];

                assert_that!(first_user.user_index).is_equal_to(user.user_index() as i64);
                assert_that!(first_user.public_keys).is_not_empty();
                assert_that!(first_user.public_keys[0].public_key.as_str())
                    .is_equal_to(user.first_key().to_string().as_str());
                assert_that!(first_user.public_keys[0].key_hash.as_str())
                    .is_equal_to(user.first_key_hash().to_string().as_str());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_single_user_multiple_public_keys() -> anyhow::Result<()> {
    let (
        suite,
        mut accounts,
        codes,
        contracts,
        validator_sets,
        _,
        dango_httpd_context,
        _,
        _db_guard,
    ) = setup_test_with_indexer(TestOption::default()).await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut test_account = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    let (pk, key_hash) = add_user_public_key(&mut suite, &contracts, &mut test_account);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, users::ResponseData>(
                    dango_httpd_context,
                    Users::build_query(users::Variables::default()),
                )
                .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.users.nodes).is_not_empty();
                let first_user = &data.users.nodes[0];

                assert_that!(first_user.user_index).is_equal_to(test_account.user_index() as i64);

                // Check the public keys (order is not guaranteed)
                let public_keys = &first_user.public_keys;
                assert_that!(public_keys.len()).is_equal_to(2);

                let has_new_key = public_keys
                    .iter()
                    .any(|p| p.public_key == pk.to_string() && p.key_hash == key_hash.to_string());
                assert_that!(has_new_key).is_true();

                let has_original_key = public_keys.iter().any(|p| {
                    p.public_key == test_account.first_key().to_string()
                        && p.key_hash == test_account.first_key_hash().to_string()
                });
                assert_that!(has_original_key).is_true();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_public_keys_by_user_index() -> anyhow::Result<()> {
    let (
        suite,
        mut accounts,
        codes,
        contracts,
        validator_sets,
        _,
        dango_httpd_context,
        _,
        _db_guard,
    ) = setup_test_with_indexer(TestOption::default()).await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let test_account = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = user::Variables {
                    user_index: test_account.user_index() as i64,
                };

                let response = call_graphql_query::<_, user::ResponseData>(
                    dango_httpd_context,
                    User::build_query(variables),
                )
                .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.user).is_some();
                let user_data = data.user.unwrap();

                assert_that!(user_data.user_index).is_equal_to(test_account.user_index() as i64);
                assert_that!(user_data.public_keys).is_not_empty();
                assert_that!(user_data.public_keys[0].public_key.as_str())
                    .is_equal_to(test_account.first_key().to_string().as_str());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
