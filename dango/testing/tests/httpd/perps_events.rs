use {
    assertor::*,
    dango_app::Indexer,
    dango_indexer_graphql_types::{
        PerpsEvents, SubscribeEventByAddresses, SubscribePerpsEvents2, perps_events,
        subscribe_event_by_addresses, subscribe_perps_events2,
    },
    dango_primitives::Addressable,
    dango_testing::{
        GraphQLCustomRequest, TestOption, build_app_service, call_graphql_query_with_context,
        call_ws_graphql_stream, create_perps_fill, pair_id, parse_graphql_subscription_response,
        setup_perps_env, setup_test_naive_with_indexer,
    },
    graphql_client::GraphQLQuery,
};

/// Query a user's perps events and verify that only `order_filled` events
/// are indexed after a limit-order match.
#[tokio::test(flavor = "multi_thread")]
async fn query_perps_events_user_lifecycle() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, dango_httpd_context, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;

    // user2 places a limit ask → user1 fills it via market buy.
    //   user2 sees: order_filled
    //   user1 sees: order_filled
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;

    suite.app.indexer.wait_for_finish().await?;

    let user2_addr = accounts.user2.address().to_string();

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                // Ascending order.
                let response = call_graphql_query_with_context::<_, perps_events::ResponseData>(
                    dango_httpd_context.clone(),
                    PerpsEvents::build_query(perps_events::Variables {
                        user_addr: Some(user2_addr.clone()),
                        sort_by: Some(perps_events::PerpsEventSortBy::BLOCK_HEIGHT_ASC),
                        pair_id: Some(pair.to_string()),
                        ..Default::default()
                    }),
                )
                .await?;

                let nodes = response.data.unwrap().perps_events.nodes;

                // Only order_filled is indexed.
                let types: Vec<&str> = nodes.iter().map(|n| n.event_type.as_str()).collect();
                assert_eq!(
                    types,
                    &["order_filled"],
                    "Unexpected event sequence (ASC): {types:?}"
                );

                // order_filled — maker side of the match at price 2000.
                assert!(
                    nodes[0].data.to_string().contains("2000"),
                    "Fill should reference price 2000, got: {}",
                    nodes[0].data
                );
                assert!(
                    nodes[0].data.get("fill_size").is_some(),
                    "Fill event should contain fill_size field"
                );

                // All events belong to user2 and the correct pair.
                for node in &nodes {
                    assert_that!(node.user_addr.as_str()).is_equal_to(user2_addr.as_str());
                    assert_that!(node.pair_id.as_str()).is_equal_to(pair.to_string().as_str());
                }

                // Descending order — same single event.
                let response_desc =
                    call_graphql_query_with_context::<_, perps_events::ResponseData>(
                        dango_httpd_context.clone(),
                        PerpsEvents::build_query(perps_events::Variables {
                            user_addr: Some(user2_addr.clone()),
                            sort_by: Some(perps_events::PerpsEventSortBy::BLOCK_HEIGHT_DESC),
                            pair_id: Some(pair.to_string()),
                            ..Default::default()
                        }),
                    )
                    .await?;

                let nodes_desc = response_desc.data.unwrap().perps_events.nodes;
                assert_that!(nodes_desc).has_length(1);
                assert_eq!(nodes_desc[0].event_type, "order_filled");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// Subscribe to `perps_events2` and verify the in-memory, validator-side feed
/// replays a fill's perps events (filtered by pair), end-to-end over the
/// WebSocket GraphQL transport.
#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_perps_events2() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, dango_httpd_context, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;

    // user2 places a limit ask → user1 fills it via market buy, emitting
    // order_persisted (placement) and order_filled (the match).
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3).await;

    suite.app.indexer.wait_for_finish().await?;

    // Replay from the oldest retained block (snapshot path — no live-timing
    // race): the fills already landed in the in-memory ring.
    let since = dango_httpd_context
        .stream_context
        .perps()
        .floor()
        .map(|h| h as i64);

    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribePerpsEvents2::build_query(subscribe_perps_events2::Variables {
            since_block_height: since,
            pair_ids: Some(vec![pair.to_string()]),
            event_types: None,
            users: None,
        }),
        "perpsEvents2",
    );

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_app_service, request_body)
                        .await?;

                // The snapshot streams one batch per matching block; read until
                // we hit the block carrying the order_filled events.
                loop {
                    let response = parse_graphql_subscription_response::<
                        subscribe_perps_events2::SubscribePerpsEvents2PerpsEvents2,
                    >(&mut framed, name)
                    .await?;

                    let batch = response.data;

                    // Every event in a pair-filtered batch is for that pair.
                    for event in &batch.events {
                        assert_eq!(event.pair_id.as_deref(), Some(pair.to_string().as_str()));
                    }

                    let Some(fill) = batch.events.iter().find(|e| e.event_type == "order_filled")
                    else {
                        continue;
                    };

                    // The fill carries the real perps payload at price 2000.
                    assert!(
                        fill.data.get("fill_size").is_some(),
                        "order_filled should carry fill_size: {}",
                        fill.data
                    );
                    assert!(
                        fill.data.to_string().contains("2000"),
                        "fill should reference price 2000: {}",
                        fill.data
                    );

                    break;
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// Deterministically reproduce the `event_by_addresses` drop bug that
/// `perps_events2` was designed to avoid.
///
/// The root cause is in the *producer*: `HookedIndexer::post_indexing` spawns
/// one `tokio::spawn` per block, so block N+1's events can be written to the
/// event cache and published before block N's. The subscription then advances
/// its watermark to N+1 on the unconditional `received_height.store(...)` and
/// discards the late N notification via the `block_height < current_received`
/// drop-guard — losing N's events silently.
///
/// We induce that exact ordering by hand: write block L+2 to the event cache
/// and publish it, then write and publish L+1. The subscription receives L+2,
/// advances past it, and drops L+1.
///
/// IGNORED: this asserts the *correct* behavior (L+1 is delivered), which the
/// current code violates. Fixing the producer ordering is out of scope for this
/// PR; un-ignore once `post_indexing` publishes in strict height order.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "reproduces a known event_by_addresses drop; the fix is out of scope for this PR"]
async fn event_by_addresses_drops_out_of_order_block() -> anyhow::Result<()> {
    use {
        dango_indexer_sql::entity::events::{EventStatus, Model as EventModel},
        dango_primitives::{Addr, FlatCommitmentStatus, Timestamp},
        sea_orm::prelude::Uuid,
        std::{collections::HashMap, sync::Arc, time::Duration},
    };

    let (mut suite, mut accounts, _, contracts, _, dango_httpd_context, _, _, _db_guard) =
        setup_test_naive_with_indexer(TestOption::default()).await;

    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000).await;
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair_id(), 2_000, 3).await;
    suite.app.indexer.wait_for_finish().await?;

    // Handles to inject events out of order: the subscription's event-cache
    // reader shares this writer's state, and it subscribes to this pubsub.
    let pubsub = dango_httpd_context.pubsub.clone();
    let event_cache = dango_httpd_context.sql_context.event_cache.clone();
    let latest = dango_httpd_context
        .stream_context
        .perps()
        .tip()
        .unwrap_or_default() as i64;

    // A fresh address with no real events, so the only deliveries are the two
    // we inject below.
    let addr = Addr::mock(0xEE);

    let fake_event = |height: i64| -> Arc<EventModel> {
        Arc::new(EventModel {
            id: Uuid::from_u128(height as u128),
            parent_id: None,
            transaction_id: None,
            message_id: None,
            created_at: Timestamp::from_seconds(0).to_naive_date_time(),
            r#type: "drop_repro".to_string(),
            method: None,
            event_status: EventStatus::Ok,
            commitment_status: FlatCommitmentStatus::Committed,
            transaction_type: 0,
            transaction_idx: 0,
            message_idx: None,
            event_idx: 0,
            data: serde_json::json!({ "height": height }),
            block_height: height,
        })
    };

    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeEventByAddresses::build_query(subscribe_event_by_addresses::Variables {
            addresses: vec![addr.to_string()],
            since_block_height: None,
        }),
        "eventByAddresses",
    );

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_app_service, request_body)
                        .await?;

                // Let the subscription register on the pubsub before publishing.
                tokio::time::sleep(Duration::from_millis(300)).await;

                let hi = latest + 2;
                let lo = latest + 1;

                // Publish L+2 (in the cache) before L+1 — the out-of-order
                // delivery the bug hinges on.
                event_cache
                    .save_events(hi as u64, HashMap::from([(addr, vec![fake_event(hi)])]))
                    .await;
                pubsub.publish(hi as u64).await.ok();

                // The subscriber processes L+2 and advances its watermark to it.
                let resp_hi = parse_graphql_subscription_response::<
                    Vec<subscribe_event_by_addresses::SubscribeEventByAddressesEventByAddresses>,
                >(&mut framed, name)
                .await?;
                assert_eq!(resp_hi.data.len(), 1);
                assert_eq!(resp_hi.data[0].block_height, hi);

                // Now L+1 lands and is published. Correct behavior: it is
                // delivered. Buggy behavior: the watermark is already at L+2, so
                // the drop-guard discards it.
                event_cache
                    .save_events(lo as u64, HashMap::from([(addr, vec![fake_event(lo)])]))
                    .await;
                pubsub.publish(lo as u64).await.ok();

                let resp_lo = tokio::time::timeout(
                    Duration::from_secs(3),
                    parse_graphql_subscription_response::<
                        Vec<
                            subscribe_event_by_addresses::SubscribeEventByAddressesEventByAddresses,
                        >,
                    >(&mut framed, name),
                )
                .await;

                match resp_lo {
                    Ok(Ok(resp)) => {
                        assert_eq!(resp.data.len(), 1);
                        assert_eq!(
                            resp.data[0].block_height, lo,
                            "expected block L+1 to be delivered"
                        );
                    },
                    _ => panic!(
                        "block L+1 was dropped — event_by_addresses lost an out-of-order block"
                    ),
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
