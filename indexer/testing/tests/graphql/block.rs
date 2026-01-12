use {
    assertor::*,
    graphql_client::{GraphQLQuery, Response},
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_client::{Block, Blocks, block, blocks},
    indexer_testing::{
        GraphQLCustomRequest,
        block::{create_block, create_blocks},
        build_app_service, call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let variables = blocks::Variables {
        after: None,
        before: None,
        first: None,
        last: None,
        sort_by: None,
    };

    let request_body = Blocks::build_query(variables);

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<blocks::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();
                assert_that!(data.blocks.nodes).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_batched_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let variables = blocks::Variables {
        after: None,
        before: None,
        first: None,
        last: None,
        sort_by: None,
    };

    let request_body = Blocks::build_query(variables.clone());
    let request_body2 = Blocks::build_query(variables);

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&vec![request_body, request_body2])
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let responses: Vec<Response<blocks::ResponseData>> =
                    serde_json::from_slice(&response)?;

                assert_that!(responses[0].data.as_ref().unwrap().blocks.nodes).is_not_empty();
                assert_that!(responses[1].data.as_ref().unwrap().blocks.nodes).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_block() -> anyhow::Result<()> {
    // NOTE: It's necessary to capture the client in a variable named `_client`
    // here. It can't be named just an underscore (`_`) or dropped (`..`).
    // Otherwise, the indexer is dropped and the test fails.
    // You can see multiple instances of this throughout this file.
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = block::Variables { height: Some(1) };
                let request_body = Block::build_query(variables);

                let app = build_app_service(httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<block::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let block = response.data.unwrap().block.unwrap();
                assert_that!(block.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_last_block() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = block::Variables { height: None };
                let request_body = Block::build_query(variables);

                let app = build_app_service(httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<block::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let block = response.data.unwrap().block.unwrap();
                assert_that!(block.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_paginate_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let blocks_count = 2;

                // Helper to paginate through all blocks
                async fn paginate_all_blocks(
                    httpd_context: indexer_httpd::context::Context,
                    sort_by: blocks::BlockSortBy,
                    first: Option<i64>,
                    last: Option<i64>,
                ) -> anyhow::Result<Vec<i64>> {
                    let mut all_heights = vec![];
                    let mut after: Option<String> = None;
                    let mut before: Option<String> = None;

                    loop {
                        let variables = blocks::Variables {
                            after: after.clone(),
                            before: before.clone(),
                            first,
                            last,
                            sort_by: Some(sort_by.clone()),
                        };

                        let request_body = Blocks::build_query(variables);
                        let app = build_app_service(httpd_context.clone());
                        let app = actix_web::test::init_service(app).await;

                        let request = actix_web::test::TestRequest::post()
                            .uri("/graphql")
                            .set_json(&request_body)
                            .to_request();

                        let response = actix_web::test::call_and_read_body(&app, request).await;
                        let response: Response<blocks::ResponseData> =
                            serde_json::from_slice(&response)?;

                        let data = response.data.unwrap();

                        match (first, last) {
                            (Some(_), None) => {
                                for node in data.blocks.nodes {
                                    all_heights.push(node.block_height);
                                }

                                if !data.blocks.page_info.has_next_page {
                                    break;
                                }
                                after = data.blocks.page_info.end_cursor;
                            },
                            (None, Some(_)) => {
                                for node in data.blocks.nodes.into_iter().rev() {
                                    all_heights.push(node.block_height);
                                }

                                if !data.blocks.page_info.has_previous_page {
                                    break;
                                }
                                before = data.blocks.page_info.start_cursor;
                            },
                            _ => break,
                        }
                    }

                    Ok(all_heights)
                }

                // 1. first with descending order
                let block_heights = paginate_all_blocks(
                    httpd_context.clone(),
                    blocks::BlockSortBy::BLOCK_HEIGHT_DESC,
                    Some(blocks_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let block_heights = paginate_all_blocks(
                    httpd_context.clone(),
                    blocks::BlockSortBy::BLOCK_HEIGHT_ASC,
                    Some(blocks_count),
                    None,
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let block_heights = paginate_all_blocks(
                    httpd_context.clone(),
                    blocks::BlockSortBy::BLOCK_HEIGHT_DESC,
                    None,
                    Some(blocks_count),
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let block_heights = paginate_all_blocks(
                    httpd_context.clone(),
                    blocks::BlockSortBy::BLOCK_HEIGHT_ASC,
                    None,
                    Some(blocks_count),
                )
                .await?
                .into_iter()
                .map(|h| h as u64)
                .collect::<Vec<_>>();

                assert_that!(block_heights).is_equal_to((1..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_block() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Block {
        block {
          blockHeight
          createdAt
          hash
          appHash
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

                // Define a simple struct to capture subscription response
                #[derive(serde::Deserialize, Debug)]
                #[serde(rename_all = "camelCase")]
                struct SubscriptionBlock {
                    block_height: i64,
                }

                // 1st response is always the existing last block
                let response =
                    parse_graphql_subscription_response::<SubscriptionBlock>(&mut framed, name)
                        .await?;

                assert_that!(response.block_height).is_equal_to(1);

                crate_block_tx.send(2).await?;

                // 2nd response
                let response =
                    parse_graphql_subscription_response::<SubscriptionBlock>(&mut framed, name)
                        .await?;

                assert_that!(response.block_height).is_equal_to(2);

                crate_block_tx.send(3).await?;

                // 3rd response
                let response =
                    parse_graphql_subscription_response::<SubscriptionBlock>(&mut framed, name)
                        .await?;

                assert_that!(response.block_height).is_equal_to(3);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
