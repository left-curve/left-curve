use {
    assertor::*,
    graphql_client::GraphQLQuery,
    grug_types::{BroadcastClientExt, Coins, Denom, ResultExt},
    indexer_client::{
        Events, SubscribeEvents, Transactions, events, subscribe_events, transactions,
    },
    indexer_testing::{
        GraphQLCustomRequest, PaginationDirection,
        block::{create_block, create_blocks},
        build_app_service, call_graphql_query, call_ws_graphql_stream, events_query,
        paginate_events, parse_graphql_subscription_response,
    },
    itertools::Itertools,
    sea_orm::{EntityTrait, PaginatorTrait},
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_events() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Events::build_query(events::Variables::default());

                let response =
                    call_graphql_query::<_, events::ResponseData, _, _, _>(app, query_body).await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();
                assert_that!(data.events.nodes).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_events_transaction_hashes() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Events::build_query(events::Variables::default());

                let response =
                    call_graphql_query::<_, events::ResponseData, _, _, _>(app, query_body).await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                let hashes: Vec<_> = data
                    .events
                    .nodes
                    .iter()
                    .filter_map(|node| node.transaction.as_ref().map(|tx| &tx.hash))
                    .collect();

                assert_that!(hashes).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_paginate_events() -> anyhow::Result<()> {
    let (httpd_context, _client, _) = create_blocks(10).await?;

    let events_total_count = indexer_sql::entity::events::Entity::find()
        .count(&httpd_context.db)
        .await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let page_size = 2;

                // 1. first with descending order
                let events = paginate_events(
                    httpd_context.clone(),
                    page_size,
                    events_query::Variables {
                        sort_by: Some(events_query::EventSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = events.iter().map(|e| e.block_height).collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 2. first with ascending order
                let events = paginate_events(
                    httpd_context.clone(),
                    page_size,
                    events_query::Variables {
                        sort_by: Some(events_query::EventSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = events.iter().map(|e| e.block_height).collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 3. last with descending order
                let events = paginate_events(
                    httpd_context.clone(),
                    page_size,
                    events_query::Variables {
                        sort_by: Some(events_query::EventSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = events.iter().map(|e| e.block_height).collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 4. last with ascending order
                let events = paginate_events(
                    httpd_context.clone(),
                    page_size,
                    events_query::Variables {
                        sort_by: Some(events_query::EventSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = events.iter().map(|e| e.block_height).collect();

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_events() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    // Use typed subscription from indexer-client
    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeEvents::build_query(subscribe_events::Variables::default()),
        "events",
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
                    grug_types::Message::transfer(
                        to,
                        Coins::one(Denom::from_str("ugrug")?, 2_000)?,
                    )?,
                    grug_types::GasOption::Predefined { gas_limit: 2000 },
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
                    Vec<subscribe_events::SubscribeEventsEvents>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_events::SubscribeEventsEvents>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_events::SubscribeEventsEvents>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_nested_events() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Transactions::build_query(transactions::Variables::default());

                let response =
                    call_graphql_query::<_, transactions::ResponseData, _, _, _>(app, query_body)
                        .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                let nested_events = data.transactions.nodes[0]
                    .nested_events
                    .as_ref()
                    .expect("Can't get nestedEvents");

                assert_that!(nested_events.len()).is_at_least(10);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
