use {
    crate::build_actix_app,
    assert_json_diff::*,
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{
        HyperlaneTestSuite, TestOption, add_account_with_existing_user, create_user_and_account,
        setup_test_with_indexer,
    },
    dango_types::{
        account::single::{QueryMsg, QuerySeenNoncesRequest},
        auth::Nonce,
        constants::dango,
    },
    graphql_client::{GraphQLQuery, Response},
    grug::{
        Addressable, Coin, Coins, Inner, Json, JsonDeExt, QuerierExt, Query, QueryBalanceRequest,
        QueryResponse, ResultExt,
    },
    grug_app::Indexer,
    grug_types::{JsonSerExt, QueryWasmSmartRequest},
    indexer_client::{Accounts, QueryApp, accounts, query_app},
    indexer_testing::{
        GraphQLCustomRequest, call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    std::collections::BTreeSet,
    tokio::{sync::mpsc, time::sleep},
};

#[tokio::test(flavor = "multi_thread")]
async fn query_accounts() -> anyhow::Result<()> {
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

    let user1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
    let user2 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let request_body = Accounts::build_query(accounts::Variables::default());

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<accounts::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();
                let nodes = &data.accounts.nodes;

                assert_that!(nodes.len()).is_equal_to(2);

                assert_that!(nodes[0].account_type).is_equal_to(accounts::AccountType::single);
                assert_that!(nodes[0].users[0].user_index).is_equal_to(user2.user_index() as i64);

                assert_that!(nodes[1].account_type).is_equal_to(accounts::AccountType::single);
                assert_that!(nodes[1].users[0].user_index).is_equal_to(user1.user_index() as i64);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_accounts_with_user_index() -> anyhow::Result<()> {
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
                let variables = accounts::Variables {
                    user_index: Some(user.user_index() as i64),
                    ..Default::default()
                };

                let request_body = Accounts::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<accounts::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.accounts.nodes).is_not_empty();
                let first_account = &data.accounts.nodes[0];

                assert_that!(first_account.account_type).is_equal_to(accounts::AccountType::single);
                assert_that!(first_account.users).is_not_empty();
                assert_that!(first_account.users[0].user_index)
                    .is_equal_to(user.user_index() as i64);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_accounts_with_wrong_user_index() -> anyhow::Result<()> {
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

    create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = accounts::Variables {
                    user_index: Some(114514), // a random user index that doesn't exist
                    ..Default::default()
                };

                let request_body = Accounts::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<accounts::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.accounts.nodes).is_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[ignore = "flaky"]
#[tokio::test(flavor = "multi_thread")]
async fn query_user_multiple_single_signature_accounts() -> anyhow::Result<()> {
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

    // Create two accounts under the same user. The two `TestAccount`'s should
    // have the same user index.
    let mut test_account1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
    let test_account2 = add_account_with_existing_user(&mut suite, &contracts, &mut test_account1);
    assert_eq!(test_account1.user_index(), test_account2.user_index());

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = accounts::Variables {
                    user_index: Some(test_account1.user_index() as i64),
                    ..Default::default()
                };

                let request_body = Accounts::build_query(variables.clone());

                // Trying to figure out a bug
                for _ in 0..10 {
                    let app = build_actix_app(dango_httpd_context.clone());
                    let app = actix_web::test::init_service(app).await;

                    let request = actix_web::test::TestRequest::post()
                        .uri("/graphql")
                        .set_json(&request_body)
                        .to_request();

                    let response = actix_web::test::call_and_read_body(&app, request).await;
                    let response: Response<accounts::ResponseData> =
                        serde_json::from_slice(&response)?;

                    let data = response.data.unwrap();

                    if data.accounts.nodes.len() == 2 {
                        break;
                    }

                    tracing::error!(
                        "Expected 2 accounts, got {:#?}. Retrying...",
                        data.accounts.nodes
                    );

                    sleep(std::time::Duration::from_millis(1000)).await;
                }

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<accounts::ResponseData> = serde_json::from_slice(&response)?;

                let data = response.data.unwrap();

                assert!(
                    data.accounts.nodes.len() == 2,
                    "Received accounts: {:#?}",
                    data.accounts.nodes
                );

                // Check first account (test_account2)
                assert_that!(data.accounts.nodes[0].account_type)
                    .is_equal_to(accounts::AccountType::single);
                assert_that!(data.accounts.nodes[0].address.as_str())
                    .is_equal_to(test_account2.address.inner().to_string().as_str());
                assert_that!(data.accounts.nodes[0].users[0].user_index)
                    .is_equal_to(test_account1.user_index() as i64);

                // Check second account (test_account1)
                assert_that!(data.accounts.nodes[1].account_type)
                    .is_equal_to(accounts::AccountType::single);
                assert_that!(data.accounts.nodes[1].address.as_str())
                    .is_equal_to(test_account1.address.inner().to_string().as_str());
                assert_that!(data.accounts.nodes[1].users[0].user_index)
                    .is_equal_to(test_account1.user_index() as i64);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_paginate_accounts() -> anyhow::Result<()> {
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

    // Create 10 accounts to paginate through
    for _ in 0..10 {
        let _user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
    }

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let accounts_count = 2;

                // Helper to paginate through all accounts
                async fn paginate_all_accounts(
                    httpd_context: dango_httpd::context::Context,
                    sort_by: accounts::AccountSortBy,
                    first: Option<i64>,
                    last: Option<i64>,
                ) -> anyhow::Result<Vec<i64>> {
                    let mut all_heights = vec![];
                    let mut after: Option<String> = None;
                    let mut before: Option<String> = None;

                    loop {
                        let variables = accounts::Variables {
                            after: after.clone(),
                            before: before.clone(),
                            first,
                            last,
                            sort_by: Some(sort_by.clone()),
                            ..Default::default()
                        };

                        let request_body = Accounts::build_query(variables);
                        let app = build_actix_app(httpd_context.clone());
                        let app = actix_web::test::init_service(app).await;

                        let request = actix_web::test::TestRequest::post()
                            .uri("/graphql")
                            .set_json(&request_body)
                            .to_request();

                        let response = actix_web::test::call_and_read_body(&app, request).await;
                        let response: Response<accounts::ResponseData> =
                            serde_json::from_slice(&response)?;

                        let data = response.data.unwrap();

                        match (first, last) {
                            (Some(_), None) => {
                                for node in data.accounts.nodes {
                                    all_heights.push(node.created_block_height);
                                }

                                if !data.accounts.page_info.has_next_page {
                                    break;
                                }
                                after = data.accounts.page_info.end_cursor;
                            },
                            (None, Some(_)) => {
                                for node in data.accounts.nodes.into_iter().rev() {
                                    all_heights.push(node.created_block_height);
                                }

                                if !data.accounts.page_info.has_previous_page {
                                    break;
                                }
                                before = data.accounts.page_info.start_cursor;
                            },
                            _ => break,
                        }
                    }

                    Ok(all_heights)
                }

                // 1. first with descending order
                let block_heights = paginate_all_accounts(
                    dango_httpd_context.clone(),
                    accounts::AccountSortBy::BLOCK_HEIGHT_DESC,
                    Some(accounts_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                // Nonce 1: register first user
                // Nonce 2: fund first user
                // Nonce 3: register second user
                // Nonce 4: fund second user
                // etc...
                // The expected nonces where the accounts are created are 1, 3, 5, 7, ...
                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2 - 1).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_all_accounts(
                    dango_httpd_context.clone(),
                    accounts::AccountSortBy::BLOCK_HEIGHT_ASC,
                    Some(accounts_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2 - 1).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_all_accounts(
                    dango_httpd_context.clone(),
                    accounts::AccountSortBy::BLOCK_HEIGHT_DESC,
                    None,
                    Some(accounts_count),
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2 - 1).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_all_accounts(
                    dango_httpd_context.clone(),
                    accounts::AccountSortBy::BLOCK_HEIGHT_ASC,
                    None,
                    Some(accounts_count),
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights)
                    .is_equal_to((1..=10).map(|x| x * 2 - 1).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_accounts() -> anyhow::Result<()> {
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

    let _test_account = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

    suite.app.indexer.wait_for_finish().await?;

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Accounts {
        accounts {
          id
          address
          accountIndex
          accountType
          createdAt
          createdBlockHeight
          createdTxHash
          users { userIndex }
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
        while let Some(_idx) = rx.recv().await {
            let _test_account =
                create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
        }
        Ok::<(), anyhow::Error>(())
    });

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<Vec<entity::accounts::Model>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.created_block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1]);

                create_account_tx.send(2).await.unwrap();

                // 2nd response
                let response = parse_graphql_subscription_response::<Vec<entity::accounts::Model>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.created_block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![3]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_accounts_with_user_index() -> anyhow::Result<()> {
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

    let mut test_account1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);
    let user_index = test_account1.user_index();

    suite.app.indexer.wait_for_finish().await?;

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Accounts($userIndex: Int) {
        accounts(userIndex: $userIndex) {
          id
          address
          accountIndex
          accountType
          createdAt
          createdBlockHeight
          users { userIndex }
        }
      }
    "#;

    let variables = serde_json::json!({
        "userIndex": user_index,
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
        while let Some(_idx) = rx.recv().await {
            // Create a new account with a new user index, to see if the subscription filters it out
            let _test_account =
                create_user_and_account(&mut suite, &mut accounts, &contracts, &codes);

            // Create a new account with the original user
            let _test_account2 =
                add_account_with_existing_user(&mut suite, &contracts, &mut test_account1);

            suite.app.indexer.wait_for_finish().await?;
        }
        Ok::<(), anyhow::Error>(())
    });

    let name = request_body.name;

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                let expected_data = serde_json::json!({
                    "users": [
                        {
                            "userIndex": user_index,
                        },
                    ],
                });

                // 1st response is always accounts from the last block if any
                let response = parse_graphql_subscription_response::<Vec<serde_json::Value>>(
                    &mut framed,
                    name,
                )
                .await?;

                let account = response
                    .data
                    .first()
                    .expect("Expected at least one account");

                assert_json_include!(actual: account, expected: expected_data);

                create_account_tx.send(2).await.unwrap();

                // 2nd response
                let response = parse_graphql_subscription_response::<Vec<serde_json::Value>>(
                    &mut framed,
                    name,
                )
                .await?;

                let account = response
                    .data
                    .first()
                    .expect("Expected at least one account");

                assert_json_include!(actual: account, expected: expected_data);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_account_owner_nonces() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, _, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    // copied from `tracked_nonces_works``
    for _ in 0..20 {
        suite
            .transfer(
                &mut accounts.owner,
                accounts.user1.address(),
                Coins::one(dango::DENOM.clone(), 123).unwrap(),
            )
            .should_succeed();
    }

    suite.app.indexer.wait_for_finish().await?;

    suite
        .query_wasm_smart(accounts.owner.address(), QuerySeenNoncesRequest {})
        .should_succeed_and_equal((0..20).collect());

    let body_request = grug_types::Query::WasmSmart(QueryWasmSmartRequest {
        contract: accounts.owner.address(),
        msg: (QueryMsg::SeenNonces {}).to_json_value()?,
    })
    .to_json_value()?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = query_app::Variables {
                    request: body_request.into_inner(),
                    ..Default::default()
                };

                let request_body = QueryApp::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<query_app::ResponseData> =
                    serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                let expected_data =
                    QueryResponse::WasmSmart((0..20).collect::<BTreeSet<Nonce>>().to_json_value()?)
                        .to_json_value()?;

                assert_json_eq!(data.query_app, expected_data);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_address_balance() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, _, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    // copied from `tracked_nonces_works``
    for _ in 0..20 {
        suite
            .transfer(
                &mut accounts.owner,
                accounts.user1.address(),
                Coins::one(dango::DENOM.clone(), 123).unwrap(),
            )
            .should_succeed();
    }

    let balance = suite
        .app
        .do_query_app(
            Query::balance(accounts.user1.address(), dango::DENOM.clone()),
            Some(20),
            false,
        )
        .unwrap()
        .into_balance();

    suite.app.indexer.wait_for_finish().await?;

    let body_request = grug_types::Query::Balance(QueryBalanceRequest {
        address: accounts.user1.address(),
        denom: dango::DENOM.clone(),
    })
    .to_json_value()?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = query_app::Variables {
                    request: body_request.into_inner(),
                    ..Default::default()
                };

                let request_body = QueryApp::build_query(variables);

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<query_app::ResponseData> =
                    serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                let httpd_balance: Coin =
                    Json::from_inner(data.query_app.get("balance").unwrap().to_owned())
                        .deserialize_json()
                        .unwrap();

                assert_that!(httpd_balance).is_equal_to(balance);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
