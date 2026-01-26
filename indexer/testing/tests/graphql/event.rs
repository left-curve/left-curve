use {
    assertor::*,
    graphql_client::GraphQLQuery,
    grug_types::{BroadcastClientExt, Coins, Denom, ResultExt},
    indexer_client::{Events, Transactions, events, transactions},
    indexer_testing::{
        GraphQLCustomRequest,
        block::{create_block, create_blocks},
        build_app_service, call_graphql_query, call_ws_graphql_stream,
        parse_graphql_subscription_response,
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
                let events_count = 2;

                // Helper to paginate through all events
                async fn paginate_all_events(
                    httpd_context: indexer_httpd::context::Context,
                    sort_by: events::EventSortBy,
                    first: Option<i64>,
                    last: Option<i64>,
                ) -> anyhow::Result<Vec<i64>> {
                    let mut all_heights = vec![];
                    let mut after: Option<String> = None;
                    let mut before: Option<String> = None;

                    loop {
                        let variables = events::Variables {
                            after: after.clone(),
                            before: before.clone(),
                            first,
                            last,
                            sort_by: Some(sort_by.clone()),
                        };

                        let app = build_app_service(httpd_context.clone());
                        let query_body = Events::build_query(variables);
                        let response =
                            call_graphql_query::<_, events::ResponseData, _, _, _>(app, query_body)
                                .await?;

                        let data = response.data.unwrap();

                        match (first, last) {
                            (Some(_), None) => {
                                for node in data.events.nodes {
                                    all_heights.push(node.block_height);
                                }

                                if !data.events.page_info.has_next_page {
                                    break;
                                }
                                after = data.events.page_info.end_cursor;
                            },
                            (None, Some(_)) => {
                                for node in data.events.nodes.into_iter().rev() {
                                    all_heights.push(node.block_height);
                                }

                                if !data.events.page_info.has_previous_page {
                                    break;
                                }
                                before = data.events.page_info.start_cursor;
                            },
                            _ => break,
                        }
                    }

                    Ok(all_heights)
                }

                // 1. first with descending order
                let block_heights = paginate_all_events(
                    httpd_context.clone(),
                    events::EventSortBy::BLOCK_HEIGHT_DESC,
                    Some(events_count),
                    None,
                )
                .await?;

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 2. first with ascending order
                let block_heights = paginate_all_events(
                    httpd_context.clone(),
                    events::EventSortBy::BLOCK_HEIGHT_ASC,
                    Some(events_count),
                    None,
                )
                .await?;

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 3. last with descending order
                let block_heights = paginate_all_events(
                    httpd_context.clone(),
                    events::EventSortBy::BLOCK_HEIGHT_DESC,
                    None,
                    Some(events_count),
                )
                .await?;

                assert_that!(block_heights.iter().copied().unique().collect::<Vec<_>>())
                    .is_equal_to((1i64..=10).collect::<Vec<_>>());

                assert_that!(block_heights.len()).is_equal_to(events_total_count as usize);

                // 4. last with ascending order
                let block_heights = paginate_all_events(
                    httpd_context.clone(),
                    events::EventSortBy::BLOCK_HEIGHT_ASC,
                    None,
                    Some(events_count),
                )
                .await?;

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

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Events {
        events {
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
                struct SubscriptionEvent {
                    block_height: i64,
                }

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<Vec<SubscriptionEvent>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<Vec<SubscriptionEvent>>(
                    &mut framed,
                    name,
                )
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<Vec<SubscriptionEvent>>(
                    &mut framed,
                    name,
                )
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
