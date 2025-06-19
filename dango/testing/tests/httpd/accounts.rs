use {
    super::build_actix_app,
    assert_json_diff::*,
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{
        HyperlaneTestSuite, add_account_with_existing_user, create_user_and_account,
        setup_test_with_indexer,
    },
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse, call_graphql, call_ws_graphql_stream,
        parse_graphql_subscription_response,
    },
    serde_json::json,
    tokio::sync::mpsc,
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
                        "accountType": "spot",
                        "users": [
                            {
                                "username": user2.username.to_string(),
                            },
                        ],
                    },
                    {
                        "accountType": "spot",
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
                    "accountType": "spot",
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
                        "accountType": "spot",
                        "createdBlockHeight": 3,
                        "address": test_account2.address.inner().to_string(),
                        "users": [
                            {
                                "username": "user",
                            },
                        ],
                    },
                    {
                        "accountType": "spot",
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_paginate_accounts() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    // Create 10 accounts to paginate through
    for idx in 0..10 {
        let _user = create_user_and_account(
            &mut suite,
            &mut accounts,
            &contracts,
            &codes,
            &format!("foo{idx}"),
        );
    }

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      query Accounts($after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
        accounts(after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
          nodes {
            id
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
          }
          edges { node { id address accountIndex accountType createdAt createdBlockHeight } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut after: Option<String> = None;
                let mut before: Option<String> = None;
                let accounts_count = 2;

                let mut block_heights = vec![];

                // 1. first with descending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "first": accounts_count,
                          "sortBy": "BLOCK_HEIGHT_DESC",
                          "after": after,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "accounts",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::accounts::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in &response.data.edges {
                        block_heights.push(edge.node.created_block_height);
                    }

                    if !response.data.page_info.has_next_page {
                        break;
                    }

                    after = Some(response.data.page_info.end_cursor);
                }

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2).rev().collect::<Vec<_>>());

                after = None;
                block_heights.clear();

                // 2. first with ascending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "first": accounts_count,
                          "sortBy": "BLOCK_HEIGHT_ASC",
                          "after": after,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "accounts",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::accounts::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in &response.data.edges {
                        block_heights.push(edge.node.created_block_height);
                    }

                    if !response.data.page_info.has_next_page {
                        break;
                    }

                    after = Some(response.data.page_info.end_cursor);
                }

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2).collect::<Vec<_>>());

                block_heights.clear();

                // 3. last with descending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "last": accounts_count,
                          "sortBy": "BLOCK_HEIGHT_DESC",
                          "before": before,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "accounts",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::accounts::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in response.data.edges.iter().rev() {
                        block_heights.push(edge.node.created_block_height);
                    }

                    if !response.data.page_info.has_previous_page {
                        break;
                    }

                    before = Some(response.data.page_info.start_cursor);
                }

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2).collect::<Vec<_>>());

                block_heights.clear();
                before = None;

                // 4. last with ascending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "last": accounts_count,
                          "sortBy": "BLOCK_HEIGHT_ASC",
                          "before": before,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "accounts",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::accounts::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in response.data.edges.iter().rev() {
                        block_heights.push(edge.node.created_block_height);
                    }

                    if !response.data.page_info.has_previous_page {
                        break;
                    }

                    before = Some(response.data.page_info.start_cursor);
                }

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_accounts() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let _test_account =
        create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      subscription Accounts {
      accounts {
            id
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "accounts",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (create_account_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(idx) = rx.recv().await {
            let _test_account = create_user_and_account(
                &mut suite,
                &mut accounts,
                &contracts,
                &codes,
                &format!("foo{idx}"),
            );

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }
        Ok::<(), anyhow::Error>(())
    });

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_actix_app, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::accounts::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.created_block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![2]);

                create_account_tx.send(2).await.unwrap();

                // 2nd response
                let (_, response) = parse_graphql_subscription_response::<
                    Vec<entity::accounts::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.created_block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![4]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_accounts_with_username() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, httpd_context) =
        setup_test_with_indexer();
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut test_account1 =
        create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
      subscription Accounts($username: String) {
      accounts(username: $username) {
            id
            address
            accountIndex
            accountType
            createdAt
            createdBlockHeight
            users { username }
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

    // Can't call this from LocalSet so using channels instead.
    let (create_account_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(idx) = rx.recv().await {
            // Create a new account with a new username
            let _test_account = create_user_and_account(
                &mut suite,
                &mut accounts,
                &contracts,
                &codes,
                &format!("foo{idx}"),
            );

            // Create a new account with the same username
            let _test_account2 =
                add_account_with_existing_user(&mut suite, &contracts, &mut test_account1);

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }
        Ok::<(), anyhow::Error>(())
    });

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_actix_app, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::accounts::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.created_block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![2]);

                create_account_tx.send(2).await.unwrap();

                // 2nd response
                let (_, response) = parse_graphql_subscription_response::<
                    Vec<entity::accounts::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.created_block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![5]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
