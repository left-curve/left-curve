use {
    crate::{build_actix_app, paginate_models},
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::{HyperlaneTestSuite, create_user_and_account, setup_test_with_indexer},
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
        constants::usdc,
    },
    grug::{Addressable, Coins, Message, NonEmpty, ResultExt},
    grug_app::Indexer,
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse, call_paginated_graphql, call_ws_graphql_stream,
        parse_graphql_subscription_response,
    },
    itertools::Itertools,
    serde_json::json,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transfer_and_accounts() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(false).await;

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
        },
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Transfers($block_height: Int!) {
        transfers(blockHeight: $block_height) {
          nodes {
            id
            idx
            blockHeight
            txHash
            fromAddress
            toAddress
            amount
            denom
            createdAt
            accounts { address users { username }}
            fromAccount { address users { username }}
            toAccount { address users { username }}
          }
          edges { node { id idx blockHeight txHash fromAddress toAddress amount denom createdAt accounts { address users { username }} fromAccount { address users { username }} toAccount { address users { username }} } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let variables = serde_json::json!({
        "block_height": 1,
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context);

                let response: PaginatedResponse<entity::transfers::Model> =
                    call_paginated_graphql(app, request_body).await?;

                assert_that!(response.edges).has_length(2);

                assert_that!(
                    response
                        .edges
                        .iter()
                        .map(|t| t.node.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1, 1]);

                assert_that!(
                    response
                        .edges
                        .iter()
                        .map(|t| t.node.amount.as_str())
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000", "100000000"]);

                response.edges.iter().for_each(|edge| {
                    assert!(
                        !edge.node.tx_hash.is_empty(),
                        "Transaction hash should not be empty."
                    );
                });

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_transfers_with_username() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_httpd_context, _) =
        setup_test_with_indexer(false).await;

    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut user1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "foo");
    let user2 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "foo2");

    suite
        .transfer(
            &mut user1,
            user2.address(),
            Coins::one(usdc::DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Transfers($username: String) {
        transfers(username: $username) {
          nodes {
            id
            idx
            blockHeight
            txHash
            fromAddress
            toAddress
            amount
            denom
            createdAt
            accounts { address users { username }}
            fromAccount { address users { username }}
            toAccount { address users { username }}
          }
          edges { node { id idx blockHeight txHash fromAddress toAddress amount denom createdAt accounts { address users { username }} fromAccount { address users { username }} toAccount { address users { username }} } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let variables = serde_json::json!({
        "username": user1.username,
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context);

                let response: PaginatedResponse<serde_json::Value> =
                    call_paginated_graphql(app, request_body).await?;

                assert_that!(response.edges).has_length(1);

                assert_that!(
                    response
                        .edges
                        .iter()
                        .flat_map(|t| t.node.get("amount").unwrap().as_str())
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100"]);

                assert_that!(
                    response
                        .edges
                        .iter()
                        .flat_map(|t| t
                            .node
                            .get("fromAccount")
                            .unwrap()
                            .as_object()
                            .and_then(|o| o.get("users"))
                            .and_then(|u| u.as_array())
                            .and_then(|a| a.first())
                            .and_then(|u| u.get("username"))
                            .and_then(|u| u.as_str()))
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["foo"]);

                assert_that!(
                    response
                        .edges
                        .iter()
                        .flat_map(|t| t
                            .node
                            .get("toAccount")
                            .unwrap()
                            .as_object()
                            .and_then(|o| o.get("users"))
                            .and_then(|u| u.as_array())
                            .and_then(|a| a.first())
                            .and_then(|u| u.get("username"))
                            .and_then(|u| u.as_str()))
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["foo2"]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_transfers_with_wrong_username() -> anyhow::Result<()> {
    let (suite, mut accounts, codes, contracts, validator_sets, _, dango_httpd_context, _) =
        setup_test_with_indexer(false).await;

    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let mut user1 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "foo");
    let user2 = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "foo2");

    suite
        .transfer(
            &mut user1,
            user2.address(),
            Coins::one(usdc::DENOM.clone(), 100).unwrap(),
        )
        .should_succeed();

    let graphql_query = r#"
      query Transfers($username: String) {
        transfers(username: $username) {
          nodes {
            id
            idx
            blockHeight
            txHash
            fromAddress
            toAddress
            amount
            denom
            createdAt
            accounts { address users { username }}
            fromAccount { address users { username }}
            toAccount { address users { username }}
          }
          edges { node { id idx blockHeight txHash fromAddress toAddress amount denom createdAt accounts { address users { username }} fromAccount { address users { username }} toAccount { address users { username }} } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let variables = serde_json::json!({
        "username": "wrong_username",
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context);

                let response: PaginatedResponse<serde_json::Value> =
                    call_paginated_graphql(app, request_body).await?;

                assert_that!(response.edges).is_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_paginate_transfers() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(false).await;

    // Create 10 transfers to paginate through
    for _ in 0..10 {
        // Copied from benchmarks.rs
        let msgs = vec![Message::execute(
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterAccount {
                params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
            },
            Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
        )?];

        suite
            .send_messages_with_gas(
                &mut accounts.user1,
                50_000_000,
                NonEmpty::new_unchecked(msgs),
            )
            .should_succeed();
    }

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Transfers($after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
        transfers(after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
          nodes {
            id
            idx
            blockHeight
            txHash
            fromAddress
            toAddress
            amount
            denom
            createdAt
            accounts { address users { username }}
            fromAccount { address users { username }}
            toAccount { address users { username }}
          }
          edges { node { id idx blockHeight txHash fromAddress toAddress amount denom createdAt accounts { address users { username }} fromAccount { address users { username }} toAccount { address users { username }} } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let transfers_count = 2;

                // 1. first with descending order
                let block_heights = paginate_models::<entity::transfers::Model>(
                    dango_httpd_context.clone(),
                    graphql_query,
                    "transfers",
                    "BLOCK_HEIGHT_DESC",
                    Some(transfers_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_models::<entity::transfers::Model>(
                    dango_httpd_context.clone(),
                    graphql_query,
                    "transfers",
                    "BLOCK_HEIGHT_ASC",
                    Some(transfers_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_models::<entity::transfers::Model>(
                    dango_httpd_context.clone(),
                    graphql_query,
                    "transfers",
                    "BLOCK_HEIGHT_DESC",
                    None,
                    Some(transfers_count),
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_models::<entity::transfers::Model>(
                    dango_httpd_context.clone(),
                    graphql_query,
                    "transfers",
                    "BLOCK_HEIGHT_ASC",
                    None,
                    Some(transfers_count),
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_transfers() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(false).await;

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
        },
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      subscription Transfer {
        transfers {
          id
          idx
          createdAt
          blockHeight
          txHash
          fromAddress
          toAddress
          amount
          denom
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(_idx) = rx.recv().await {
            let msgs = vec![Message::transfer(
                accounts.user2.address(),
                Coins::one(usdc::DENOM.clone(), 123).unwrap(),
            )?];

            suite
                .send_messages_with_gas(
                    &mut accounts.user1,
                    50_000_000,
                    NonEmpty::new_unchecked(msgs),
                )
                .should_succeed();

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
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last block
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1, 1]);

                crate_block_tx.send(2).await.unwrap();

                // 2nd response
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![2]);

                crate_block_tx.send(3).await.unwrap();

                // 3rd response
                let (_, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![3]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_transfers_with_filter() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(false).await;

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
        },
        Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    let graphql_query = r#"
      subscription Transfer($address: String) {
        transfers(address: $address) {
          id
          idx
          createdAt
          blockHeight
          txHash
          fromAddress
          toAddress
          amount
          denom
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transfers",
        query: graphql_query,
        variables: json!({"address": accounts.user1.address})
            .as_object()
            .unwrap()
            .to_owned(),
    };

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (create_block_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(_idx) = rx.recv().await {
            // Copied from benchmarks.rs
            let msgs = vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterAccount {
                    params: AccountParams::Spot(single::Params::new(
                        accounts.user1.username.clone(),
                    )),
                },
                Coins::one(usdc::DENOM.clone(), 100_000_000).unwrap(),
            )?];

            suite
                .send_messages_with_gas(
                    &mut accounts.user1,
                    50_000_000,
                    NonEmpty::new_unchecked(msgs),
                )
                .should_succeed();

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
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last block
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                // 1st transfer because we filter on one address
                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.amount)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000".to_string()]);

                create_block_tx.send(2).await.unwrap();

                // 2nd response
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.amount)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000".to_string()]);

                create_block_tx.send(3).await.unwrap();

                // 3rd response
                let (_, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.amount)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000".to_string()]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
