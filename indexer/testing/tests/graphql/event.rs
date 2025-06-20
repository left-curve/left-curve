use {
    assertor::*,
    grug_types::{BroadcastClientExt, Coins, Denom, ResultExt},
    indexer_sql::entity,
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse,
        block::{create_block, create_blocks},
        build_app_service, call_graphql, call_ws_graphql_stream,
        graphql::paginate_models,
        parse_graphql_subscription_response,
    },
    itertools::Itertools,
    sea_orm::{EntityTrait, PaginatorTrait},
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_events() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Events {
        events {
          nodes {
            id
            parentId
            transactionId
            messageId
            blockHeight
            createdAt
            eventIdx
            type
            method
            eventStatus
            commitmentStatus
            transactionType
            transactionIdx
            messageIdx
            eventIdx
            data
          }
          edges {
            node {
              id
              parentId
              transactionId
              messageId
              blockHeight
              createdAt
              type
              method
              eventStatus
              commitmentStatus
              transactionType
              transactionIdx
              messageIdx
              data
              eventIdx
            }
            cursor
          }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "events",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<PaginatedResponse<entity::events::Model>, _, _, _>(
                    app,
                    request_body,
                )
                .await?;

                assert_that!(response.data.edges).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_events_transaction_hashes() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Events {
        events {
          nodes {
            transaction { hash }
            blockHeight
            createdAt
            type
            method
            eventStatus
            commitmentStatus
            data
          }
          edges {
            node {
              transaction { hash }
              blockHeight
              createdAt
              type
              method
              eventStatus
              commitmentStatus
              data
            }
            cursor
          }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "events",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<PaginatedResponse<serde_json::Value>, _, _, _>(
                    app,
                    request_body,
                )
                .await?;

                let hashes = response
                    .data
                    .edges
                    .iter()
                    .flat_map(|edge| edge.node.get("transaction").and_then(|tx| tx.get("hash")))
                    .collect::<Vec<_>>();

                assert_that!(hashes).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_paginate_events() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let events_total_count = entity::events::Entity::find()
        .count(&httpd_context.db)
        .await?;

    let graphql_query = r#"
      query Events($after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
        events(after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
          nodes {
            id
            parentId
            transactionId
            messageId
            blockHeight
            createdAt
            eventIdx
            type
            method
            eventStatus
            commitmentStatus
            transactionType
            transactionIdx
            messageIdx
            eventIdx
            data
          }
          edges {
            node {
              id
              parentId
              transactionId
              messageId
              blockHeight
              createdAt
              type
              method
              eventStatus
              commitmentStatus
              transactionType
              transactionIdx
              messageIdx
              data
              eventIdx
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
                let events_count = 2;

                // 1. first with descending order
                let block_heights = paginate_models::<entity::events::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "events",
                    "BLOCK_HEIGHT_DESC",
                    Some(events_count),
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

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 2. first with ascending order
                let block_heights = paginate_models::<entity::events::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "events",
                    "BLOCK_HEIGHT_ASC",
                    Some(events_count),
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

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 3. last with descending order
                let block_heights = paginate_models::<entity::events::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "events",
                    "BLOCK_HEIGHT_DESC",
                    None,
                    Some(events_count),
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

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 4. last with ascending order
                let block_heights = paginate_models::<entity::events::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "events",
                    "BLOCK_HEIGHT_ASC",
                    None,
                    Some(events_count),
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

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_events() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Events {
        events {
          id
          parentId
          transactionId
          messageId
          transactionType
          transactionIdx
          messageIdx
          eventIdx
          blockHeight
          createdAt
          type
          method
          eventStatus
          commitmentStatus
          data
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "events",
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
                    grug_types::Message::transfer(
                        to,
                        Coins::one(Denom::from_str("ugrug")?, 2_000)?,
                    )?,
                    grug_types::GasOption::Predefined { gas_limit: 2000 },
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
                    parse_graphql_subscription_response::<Vec<entity::events::Model>>(framed, name)
                        .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(2).await?;

                // 2nd response
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<entity::events::Model>>(framed, name)
                        .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) =
                    parse_graphql_subscription_response::<Vec<entity::events::Model>>(framed, name)
                        .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_nested_events() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Transactions {
        transactions {
          nodes {
            blockHeight
            sender
            hash
            hasSucceeded
            nestedEvents
          }
          edges { node { blockHeight sender hash hasSucceeded nestedEvents } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transactions",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<PaginatedResponse<serde_json::Value>, _, _, _>(
                    app,
                    request_body,
                )
                .await?;

                let nested_events: &str = response.data.edges[0]
                    .node
                    .get("nestedEvents")
                    .and_then(|s| s.as_str())
                    .expect("Can't get nestedEvents");

                assert_that!(nested_events.len()).is_at_least(10);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
