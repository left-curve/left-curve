use {
    super::build_actix_app,
    assertor::*,
    dango_testing::{
        HyperlaneTestSuite, TestOption, add_user_public_key, create_user_and_account,
        setup_test_with_indexer,
    },
    graphql_client::{GraphQLQuery, Response},
    grug_app::Indexer,
    indexer_client::{User, Users, user, users},
    std::collections::HashMap,
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
                let request_body = Users::build_query(users::Variables::default());

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<users::ResponseData> = serde_json::from_slice(&response)?;

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
                let request_body = Users::build_query(users::Variables::default());

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<users::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.users.nodes).is_not_empty();
                let first_user = &data.users.nodes[0];

                assert_that!(first_user.user_index).is_equal_to(test_account.user_index() as i64);

                // Manually check the public keys because the order is not guaranteed
                let received_public_keys: Vec<HashMap<String, String>> = first_user
                    .public_keys
                    .iter()
                    .map(|pk| {
                        let mut map = HashMap::new();
                        map.insert("publicKey".to_string(), pk.public_key.clone());
                        map.insert("keyHash".to_string(), pk.key_hash.clone());
                        map
                    })
                    .collect();

                assert_that!(received_public_keys).contains(
                    serde_json::from_value::<HashMap<String, String>>(serde_json::json!({
                        "publicKey": pk.to_string(),
                        "keyHash": key_hash.to_string(),
                    }))
                    .unwrap()
                    .clone(),
                );

                assert_that!(received_public_keys).contains(
                    serde_json::from_value::<HashMap<String, String>>(serde_json::json!({
                        "publicKey": test_account.first_key().to_string(),
                        "keyHash": test_account.first_key_hash().to_string()
                    }))
                    .unwrap()
                    .clone(),
                );

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

                let request_body = User::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<user::ResponseData> = serde_json::from_slice(&response)?;

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
