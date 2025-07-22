use {
    super::build_actix_app,
    assert_json_diff::*,
    assertor::*,
    dango_testing::{
        add_user_public_key, create_user_and_account, setup_test_with_indexer, HyperlaneTestSuite, TestOption
    },
    grug_app::Indexer,
    indexer_testing::{call_graphql, GraphQLCustomRequest, PaginatedResponse},
    std::collections::HashMap,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_user() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Users {
      users {
          nodes {
            username
            publicKeys { publicKey keyHash }
          }
          edges { node { username publicKeys { publicKey keyHash } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "users",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context);

                let response = call_graphql::<PaginatedResponse<serde_json::Value>, _, _, _>(
                    app,
                    request_body,
                )
                .await?;

                let expected_data = serde_json::json!({
                    "username": user.username.to_string(),
                    "publicKeys": [
                        {
                            "publicKey": user.first_key().to_string(),
                            "keyHash": user.first_key_hash().to_string(),
                        },
                    ],
                });

                assert_json_include!(actual: response.data.edges[0].node, expected: expected_data);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_single_user_multiple_public_keys() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut test_account =
        create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    let (pk, key_hash) = add_user_public_key(&mut suite, &contracts, &mut test_account);

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Users {
      users {
          nodes {
            username
            publicKeys { publicKey keyHash }
          }
          edges { node { username publicKeys { publicKey keyHash } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "users",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context);

                let response = call_graphql::<PaginatedResponse<serde_json::Value>, _, _, _>(
                    app,
                    request_body,
                )
                .await?;

                let expected_data = serde_json::json!({
                    "username": test_account.username.to_string()
                });

                assert_json_include!(actual: response.data.edges[0].node, expected: expected_data);

                let received_public_keys: Vec<HashMap<String, String>> = serde_json::from_value(
                    response.data.edges[0]
                        .node
                        .as_object()
                        .and_then(|o| o.get("publicKeys"))
                        .unwrap()
                        .clone(),
                )
                .unwrap();

                // Manually check the public keys because the order is not guaranteed

                assert_that!(received_public_keys).contains(
                    serde_json::from_value::<HashMap<String, String>>(
                        serde_json::json!({"publicKey": pk.to_string(),
                            "keyHash": key_hash.to_string(),}),
                    )
                    .unwrap()
                    .clone(),
                );

                assert_that!(received_public_keys).contains(
                    serde_json::from_value::<HashMap<String, String>>(
                        serde_json::json!({"publicKey": test_account.first_key().to_string(),
                            "keyHash": test_account.first_key_hash().to_string()}),
                    )
                    .unwrap()
                    .clone(),
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
