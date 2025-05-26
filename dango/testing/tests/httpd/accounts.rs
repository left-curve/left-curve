use {
    super::build_actix_app,
    assert_json_diff::*,
    assertor::*,
    dango_testing::{
        HyperlaneTestSuite, add_account_with_existing_user, create_user_and_account,
        setup_test_with_indexer,
    },
    indexer_testing::{GraphQLCustomRequest, PaginatedResponse, call_graphql},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_accounts() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "foo");
    let user2 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "bar");

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      query Accounts {
      accounts {
          nodes {
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
          }
          edges { node { address accountIndex accountType createdAt createdBlockHeight users { username } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "accounts",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<serde_json::Value>>(app, request_body).await?;

                let received_accounts = response
                    .data
                    .edges
                    .into_iter()
                    .map(|e| e.node)
                    .collect::<Vec<_>>();

                let expected_data = serde_json::json!([
                    {
                        "accountType": "SPOT",
                        "users": [
                            {
                                "username": user2.username.to_string(),
                            },
                        ],
                    },
                    {
                        "accountType": "SPOT",
                        "users": [
                            {
                                "username": user1.username.to_string(),
                            },
                        ],
                    },
                ]);

                assert_json_include!(actual: received_accounts, expected: expected_data);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_accounts_with_username() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      query Accounts($username: String) {
      accounts(username: $username) {
          nodes {
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
          }
          edges { node { address accountIndex accountType createdAt createdBlockHeight users { username } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let variables = serde_json::json!({
        "username": user.username.to_string(),
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "accounts",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<serde_json::Value>>(app, request_body).await?;

                let expected_data = serde_json::json!({
                    "accountType": "SPOT",
                    "users": [
                        {
                            "username": user.username.to_string(),
                        }
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
async fn query_accounts_with_wrong_username() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      query Accounts($username: String) {
      accounts(username: $username) {
          nodes {
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
          }
          edges { node { address accountIndex accountType createdAt createdBlockHeight users { username } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let variables = serde_json::json!({
        "username": "foo",
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "accounts",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(httpd_context);

                let response = call_graphql::<serde_json::Value>(app, request_body).await?;

                let nodes = response
                    .data
                    .as_object()
                    .and_then(|c| c.get("nodes"))
                    .and_then(|c| c.as_array())
                    .expect("Failed to get nodes")
                    .iter()
                    .collect::<Vec<_>>();

                assert_that!(nodes).is_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_user_multiple_spot_accounts() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut test_account1 =
        create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    let test_account2 = add_account_with_existing_user(&mut suite, &contracts, &mut test_account1);

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      query Accounts($username: String) {
      accounts(username: $username) {
          nodes {
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
          }
          edges { node { address accountIndex accountType createdAt createdBlockHeight users { username } }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let variables = serde_json::json!({
        "username": "user",
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "accounts",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<serde_json::Value>>(app, request_body).await?;

                let received_accounts = response
                    .data
                    .edges
                    .into_iter()
                    .map(|e| e.node)
                    .collect::<Vec<_>>();

                let expected_accounts = serde_json::json!([
                    {
                        "accountType": "SPOT",
                        "createdBlockHeight": 3,
                        "address": test_account2.address.inner().to_string(),
                        "users": [
                            {
                                "username": "user",
                            },
                        ],
                    },
                    {
                        "accountType": "SPOT",
                        "createdBlockHeight": 2,
                        "address": test_account1.address.inner().to_string(),
                        "users": [
                            {
                                "username": "user",
                            },
                        ],
                    }
                ]);

                assert_json_include!(actual: received_accounts, expected: expected_accounts);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
