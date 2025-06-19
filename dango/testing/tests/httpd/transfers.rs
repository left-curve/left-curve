use {
    super::build_actix_app,
    assertor::*,
    dango_indexer_sql::entity,
    dango_testing::setup_test_with_indexer,
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
        constants::usdc,
    },
    grug::{Addressable, Coins, Message, NonEmpty, ResultExt},
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse, call_graphql, call_ws_graphql_stream,
        parse_graphql_subscription_response,
    },
    itertools::Itertools,
    serde_json::json,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transfer_and_accounts() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, httpd_context) = setup_test_with_indexer();

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

    suite.app.indexer.wait_for_finish();

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
                let app = build_actix_app(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<entity::transfers::Model>>(app, request_body)
                        .await?;

                assert_that!(response.data.edges).has_length(2);

                assert_that!(
                    response
                        .data
                        .edges
                        .iter()
                        .map(|t| t.node.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1, 1]);

                assert_that!(
                    response
                        .data
                        .edges
                        .iter()
                        .map(|t| t.node.amount.as_str())
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec!["100000000", "100000000"]);

                response.data.edges.iter().for_each(|edge| {
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
async fn graphql_paginate_transfers() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, httpd_context) = setup_test_with_indexer();

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

    suite.app.indexer.wait_for_finish();

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
                let mut after: Option<String> = None;
                let mut before: Option<String> = None;
                let transfers_count = 2;

                let mut block_heights = vec![];

                // 1. first with descending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "first": transfers_count,
                          "sortBy": "BLOCK_HEIGHT_DESC",
                          "after": after,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "transfers",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::transfers::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in &response.data.edges {
                        block_heights.push(edge.node.block_height);
                    }

                    if !response.data.page_info.has_next_page {
                        break;
                    }

                    after = Some(response.data.page_info.end_cursor);
                }

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).rev().collect::<Vec<_>>());

                after = None;
                block_heights.clear();

                // 2. first with ascending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "first": transfers_count,
                          "sortBy": "BLOCK_HEIGHT_ASC",
                          "after": after,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "transfers",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::transfers::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in &response.data.edges {
                        block_heights.push(edge.node.block_height);
                    }

                    if !response.data.page_info.has_next_page {
                        break;
                    }

                    after = Some(response.data.page_info.end_cursor);
                }

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).collect::<Vec<_>>());

                block_heights.clear();

                // 3. last with descending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "last": transfers_count,
                          "sortBy": "BLOCK_HEIGHT_DESC",
                          "before": before,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "transfers",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::transfers::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in response.data.edges.iter().rev() {
                        block_heights.push(edge.node.block_height);
                    }

                    if !response.data.page_info.has_previous_page {
                        break;
                    }

                    before = Some(response.data.page_info.start_cursor);
                }

                assert_that!(
                    block_heights
                        .clone()
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>()
                )
                .is_equal_to((1..=10).collect::<Vec<_>>());

                block_heights.clear();
                before = None;

                // 4. last with ascending order
                loop {
                    let app = build_actix_app(httpd_context.clone());

                    let variables = json!({
                          "last": transfers_count,
                          "sortBy": "BLOCK_HEIGHT_ASC",
                          "before": before,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "transfers",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::transfers::Model>>(
                        app,
                        request_body,
                    )
                    .await?;

                    for edge in response.data.edges.iter().rev() {
                        block_heights.push(edge.node.block_height);
                    }

                    if !response.data.page_info.has_previous_page {
                        break;
                    }

                    before = Some(response.data.page_info.start_cursor);
                }

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
    let (mut suite, mut accounts, _, contracts, _, httpd_context) = setup_test_with_indexer();

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
                    call_ws_graphql_stream(httpd_context, build_actix_app, request_body).await?;

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
    let (mut suite, mut accounts, _, contracts, _, httpd_context) = setup_test_with_indexer();

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
                    call_ws_graphql_stream(httpd_context, build_actix_app, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::transfers::Model>,
                >(framed, name)
                .await?;

                // 1 transfer because we filter on one address
                assert_that!(
                    response
                        .data
                        .into_iter()
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![1]);

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
                        .map(|t| t.block_height)
                        .collect::<Vec<_>>()
                )
                .is_equal_to(vec![2]);

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
