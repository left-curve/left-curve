use {
    assertor::*,
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_sql::entity::{self},
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse,
        block::{create_block, create_blocks},
        build_app_service, call_graphql, call_ws_graphql_stream,
        parse_graphql_subscription_response,
    },
    serde_json::json,
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_messages() -> anyhow::Result<()> {
    let (httpd_context, _client, accounts) = create_block().await?;

    let graphql_query = r#"
      query Messages {
        messages {
          nodes {
            id
            transactionId
            orderIdx
            createdAt
            data
            blockHeight
            methodName
            contractAddr
            senderAddr
          }
          edges {
            node {
              id
              transactionId
              orderIdx
              createdAt
              data
              blockHeight
              methodName
              contractAddr
              senderAddr
            }
            cursor
          }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "messages",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<entity::messages::Model>>(app, request_body)
                        .await?;

                assert_that!(response.data.edges).has_length(1);

                assert_that!(response.data.edges[0].node.sender_addr)
                    .is_equal_to(accounts["sender"].address.to_string());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_paginate_messages() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let graphql_query = r#"
      query Messages($after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
        messages(after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
          nodes {
            id
            transactionId
            orderIdx
            createdAt
            data
            blockHeight
            methodName
            contractAddr
            senderAddr
          }
          edges {
            node {
              id
              transactionId
              orderIdx
              createdAt
              data
              blockHeight
              methodName
              contractAddr
              senderAddr
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
                let mut after: Option<String> = None;
                let mut before: Option<String> = None;
                let messages_count = 2;

                let mut block_heights = vec![];

                // 1. first with descending order
                loop {
                    let app = build_app_service(httpd_context.clone());

                    let variables = json!({
                          "first": messages_count,
                          "sortBy": "BLOCK_HEIGHT_DESC",
                          "after": after,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "messages",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::messages::Model>>(
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

                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                after = None;
                block_heights.clear();

                // 2. first with ascending order
                loop {
                    let app = build_app_service(httpd_context.clone());

                    let variables = json!({
                          "first": messages_count,
                          "sortBy": "BLOCK_HEIGHT_ASC",
                          "after": after,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "messages",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::messages::Model>>(
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

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                block_heights.clear();

                // 3. last with descending order
                loop {
                    let app = build_app_service(httpd_context.clone());

                    let variables = json!({
                          "last": messages_count,
                          "sortBy": "BLOCK_HEIGHT_DESC",
                          "before": before,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "messages",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::messages::Model>>(
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

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                block_heights.clear();
                before = None;

                // 4. last with ascending order
                loop {
                    let app = build_app_service(httpd_context.clone());

                    let variables = json!({
                          "last": messages_count,
                          "sortBy": "BLOCK_HEIGHT_ASC",
                          "before": before,
                    })
                    .as_object()
                    .unwrap()
                    .clone();

                    let request_body = GraphQLCustomRequest {
                        name: "messages",
                        query: graphql_query,
                        variables,
                    };

                    let response = call_graphql::<PaginatedResponse<entity::messages::Model>>(
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

                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_messages() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Messages {
        messages {
          id
          transactionId
          data
          blockHeight
          createdAt
          orderIdx
          methodName
          contractAddr
          senderAddr
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "messages",
        query: graphql_query,
        variables: Default::default(),
    };

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    let owner_addr = accounts["sender"].address.to_string();

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
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::messages::Model>,
                >(framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(2).await?;

                // 2st response
                let (framed, response) = parse_graphql_subscription_response::<
                    Vec<entity::messages::Model>,
                >(framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) = parse_graphql_subscription_response::<
                    Vec<entity::messages::Model>,
                >(framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
