use {
    super::build_actix_app,
    assert_json_diff::*,
    dango_testing::{
        HyperlaneTestSuite, add_user_public_key, create_user_and_account, setup_test_with_indexer,
    },
    grug_app::Indexer,
    indexer_testing::{GraphQLCustomRequest, PaginatedResponse, call_graphql},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_user() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_httpd_context) =
        setup_test_with_indexer().await;
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
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_httpd_context) =
        setup_test_with_indexer().await;
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
                    "username": test_account.username.to_string(),
                    "publicKeys": [
                        {
                            "publicKey": test_account.first_key().to_string(),
                            "keyHash": test_account.first_key_hash().to_string(),
                        },
                        {
                            "publicKey": pk.to_string(),
                            "keyHash": key_hash.to_string(),
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
