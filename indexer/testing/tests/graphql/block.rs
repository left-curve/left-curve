use {
    assertor::*,
    graphql_client::GraphQLQuery,
    grug_types::{BroadcastClientExt, Coins, Denom, GasOption, Message, ResultExt},
    indexer_client::{Block, Blocks, SubscribeBlock, block, blocks, subscribe_block},
    indexer_testing::{
        GraphQLCustomRequest, PaginationDirection,
        block::{create_block, create_blocks},
        blocks_query, build_app_service, call_batch_graphql_query, call_graphql_query,
        call_ws_graphql_stream, paginate_blocks, parse_graphql_subscription_response,
    },
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Blocks::build_query(blocks::Variables::default());

                let response =
                    call_graphql_query::<_, blocks::ResponseData, _, _, _>(app, query_body).await?;

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

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let variables = blocks::Variables::default();
                let query_bodies = vec![
                    Blocks::build_query(variables.clone()),
                    Blocks::build_query(variables),
                ];

                let responses =
                    call_batch_graphql_query::<_, blocks::ResponseData, _, _, _>(app, query_bodies)
                        .await?;

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
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Block::build_query(block::Variables { height: Some(1) });

                let response =
                    call_graphql_query::<_, block::ResponseData, _, _, _>(app, query_body).await?;

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
                let app = build_app_service(httpd_context);
                let query_body = Block::build_query(block::Variables::default());

                let response =
                    call_graphql_query::<_, block::ResponseData, _, _, _>(app, query_body).await?;

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
                let page_size = 2;

                // 1. first with descending order
                let blocks = paginate_blocks(
                    httpd_context.clone(),
                    page_size,
                    blocks_query::Variables {
                        sort_by: Some(blocks_query::BlockSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = blocks.iter().map(|b| b.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let blocks = paginate_blocks(
                    httpd_context.clone(),
                    page_size,
                    blocks_query::Variables {
                        sort_by: Some(blocks_query::BlockSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = blocks.iter().map(|b| b.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let blocks = paginate_blocks(
                    httpd_context.clone(),
                    page_size,
                    blocks_query::Variables {
                        sort_by: Some(blocks_query::BlockSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = blocks.iter().map(|b| b.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let blocks = paginate_blocks(
                    httpd_context.clone(),
                    page_size,
                    blocks_query::Variables {
                        sort_by: Some(blocks_query::BlockSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = blocks.iter().map(|b| b.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_block() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    // Use typed subscription from indexer-client
    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeBlock::build_query(subscribe_block::Variables {}),
        "block",
    );

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

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<
                    subscribe_block::SubscribeBlockBlock,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.block_height).is_equal_to(1);

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    subscribe_block::SubscribeBlockBlock,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.block_height).is_equal_to(2);

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    subscribe_block::SubscribeBlockBlock,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.block_height).is_equal_to(3);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
