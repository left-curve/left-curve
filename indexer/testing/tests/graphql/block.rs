use {
    assertor::*,
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_sql::entity,
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse,
        block::{create_block, create_blocks},
        build_app_service, call_batch_graphql, call_graphql, call_ws_graphql_stream,
        graphql::paginate_models,
        parse_graphql_subscription_response,
    },
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Blocks {
        blocks {
          nodes {
            id
            blockHeight
          }
          edges {
            node {
              id
              blockHeight
            }
            cursor
          }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "blocks",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<grug_types::Json>>(app, request_body).await?;

                assert_that!(response.data.edges).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_batched_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Blocks {
        blocks {
          nodes {
            id
            blockHeight
          }
          edges {
            node {
              id
              blockHeight
            }
            cursor
          }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "blocks",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let responses =
                    call_batch_graphql::<PaginatedResponse<grug_types::Json>>(app, vec![
                        request_body.clone(),
                        request_body,
                    ])
                    .await?;

                assert_that!(responses[0].data.edges).is_not_empty();
                assert_that!(responses[1].data.edges).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn graphql_returns_block() -> anyhow::Result<()> {
    // NOTE: It's necessary to capture the client in a variable named `_client`
    // here. It can't be named just an underscore (`_`) or dropped (`..`).
    // Otherwise, the indexer is dropped and the test fails.
    // You can see multiple instances of this throughout this file.
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block($height: Int) {
        block(height: $height) {
          id
          blockHeight
          appHash
          hash
          createdAt
          transactionsCount
        }
      }
    "#;

    let variables = serde_json::json!({
        "height": 1,
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<entity::blocks::Model>(app, request_body).await?;

                assert_that!(response.data.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_last_block() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block {
        block {
          id
          blockHeight
          appHash
          hash
          createdAt
          transactionsCount
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<entity::blocks::Model>(app, request_body).await?;
                assert_that!(response.data.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_paginate_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let graphql_query = r#"
    query Blocks($after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
        blocks(after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
          nodes {
            id
            blockHeight
            createdAt
            hash
            appHash
            transactionsCount
          }
          edges {
            node {
              id
              blockHeight
              createdAt
              hash
              appHash
              transactionsCount
            }
            cursor
          }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let blocks_count = 2;

                // 1. first with descending order
                let block_heights = paginate_models::<entity::blocks::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "blocks",
                    "BLOCK_HEIGHT_DESC",
                    Some(blocks_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_models::<entity::blocks::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "blocks",
                    "BLOCK_HEIGHT_ASC",
                    Some(blocks_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_models::<entity::blocks::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "blocks",
                    "BLOCK_HEIGHT_DESC",
                    None,
                    Some(blocks_count),
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_models::<entity::blocks::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "blocks",
                    "BLOCK_HEIGHT_ASC",
                    None,
                    Some(blocks_count),
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();
                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_block() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Block {
        block {
          id
          blockHeight
          createdAt
          hash
          appHash
          transactionsCount
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables: Default::default(),
    };

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;
            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    Message::transfer(to, Coins::one(Denom::from_str("ugrug")?, 2_000)?)?,
                    GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
                )
                .await
                .should_succeed();

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }

        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) =
                    parse_graphql_subscription_response::<entity::blocks::Model>(framed, name)
                        .await?;

                assert_that!(response.data.block_height).is_equal_to(1);

                crate_block_tx.send(2).await?;

                // 2st response
                let (framed, response) =
                    parse_graphql_subscription_response::<entity::blocks::Model>(framed, name)
                        .await?;

                assert_that!(response.data.block_height).is_equal_to(2);

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) =
                    parse_graphql_subscription_response::<entity::blocks::Model>(framed, name)
                        .await?;

                assert_that!(response.data.block_height).is_equal_to(3);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
