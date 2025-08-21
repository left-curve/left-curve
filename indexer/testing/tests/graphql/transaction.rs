use {
    assert_json_diff::assert_json_include,
    assertor::*,
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_sql::entity::{self},
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse,
        block::{create_block, create_blocks},
        build_app_service, call_graphql, call_ws_graphql_stream,
        graphql::paginate_models,
        parse_graphql_subscription_response,
    },
    serde_json::json,
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_last_block_transactions() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block {
        block {
          blockHeight
          transactions {
            blockHeight
          }
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

                let response =
                    call_graphql::<serde_json::Value, _, _, _>(app, request_body).await?;

                let expected = json!({
                    "blockHeight": 1,
                    "transactions": [
                        {
                            "blockHeight": 1,
                        }
                    ]
                });

                assert_json_include!(actual: response.data, expected: expected);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transactions() -> anyhow::Result<()> {
    let (httpd_context, _client, accounts) = create_block().await?;

    let graphql_query = r#"
      query Transactions {
        transactions {
          nodes {
            id
            blockHeight
            sender
            hash
            hasSucceeded
            createdAt
            transactionType
            transactionIdx
            data
            credential
            gasWanted
            gasUsed
            errorMessage
          }
          edges { node { id createdAt blockHeight sender hash hasSucceeded transactionType transactionIdx data credential gasWanted gasUsed errorMessage } cursor }
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

                let response =
                    call_graphql::<PaginatedResponse<entity::transactions::Model>, _, _, _>(
                        app,
                        request_body,
                    )
                    .await?;

                assert_that!(response.data.edges).has_length(1);

                assert_that!(response.data.edges[0].node.sender)
                    .is_equal_to(accounts["sender"].address.to_string());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_paginate_transactions() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let graphql_query = r#"
      query Transactions($after: String, $before: String, $first: Int, $last: Int, $sortBy: String) {
        transactions(after: $after, before: $before, first: $first, last: $last, sortBy: $sortBy) {
          nodes {
            id
            blockHeight
            sender
            hash
            hasSucceeded
            createdAt
            transactionType
            transactionIdx
            data
            credential
            gasWanted
            gasUsed
            errorMessage
          }
          edges { node { id createdAt blockHeight sender hash hasSucceeded transactionType transactionIdx data credential gasWanted gasUsed errorMessage } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let transactions_count = 2;

                // 1. first with descending order
                let block_heights = paginate_models::<entity::transactions::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "transactions",
                    "BLOCK_HEIGHT_DESC",
                    Some(transactions_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_models::<entity::transactions::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "transactions",
                    "BLOCK_HEIGHT_ASC",
                    Some(transactions_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_models::<entity::transactions::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "transactions",
                    "BLOCK_HEIGHT_DESC",
                    None,
                    Some(transactions_count),
                )
                .await?
                .into_iter()
                .map(|a| a.block_height as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_models::<entity::transactions::Model>(
                    httpd_context.clone(),
                    graphql_query,
                    "transactions",
                    "BLOCK_HEIGHT_ASC",
                    None,
                    Some(transactions_count),
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
async fn graphql_subscribe_to_transactions() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Transactions {
        transactions {
          id
          data
          credential
          blockHeight
          createdAt
          transactionType
          transactionIdx
          sender
          hash
          hasSucceeded
          errorMessage
          gasWanted
          gasUsed
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transactions",
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
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<
                    Vec<entity::transactions::Model>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    Vec<entity::transactions::Model>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    Vec<entity::transactions::Model>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
