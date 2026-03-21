use {
    crate::call_graphql_query,
    assertor::*,
    dango_testing::{
        TestOption,
        perps::{create_perps_fill, pair_id, setup_perps_env},
        setup_test_with_indexer,
    },
    dango_types::perps,
    graphql_client::GraphQLQuery,
    grug::{Addressable, Coins, QuerierExt, ResultExt},
    grug_app::Indexer,
    indexer_client::{PerpsEvents, perps_events},
    std::collections::BTreeMap,
};

/// Query a user's perps events and verify the order lifecycle is returned
/// in the correct chronological order:
///   - order_persisted (limit order placed on book)
///   - order_filled    (matched by counterparty)
///   - order_persisted (second limit order placed)
///   - order_removed   (user cancels the second order)
#[tokio::test(flavor = "multi_thread")]
async fn query_perps_events_user_lifecycle() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);

    // -----------------------------------------------------------------------
    // Flow 1: user2 places a limit ask → user1 fills it via market buy.
    //   user2 sees: order_persisted, order_filled
    //   user1 sees: order_filled
    // -----------------------------------------------------------------------
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3);

    // -----------------------------------------------------------------------
    // Flow 2: user2 places another limit ask → then cancels it.
    //   user2 sees: order_persisted, order_removed(Canceled)
    // -----------------------------------------------------------------------
    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: dango_types::Quantity::new_int(-2),
                kind: perps::OrderKind::Limit {
                    limit_price: dango_types::UsdPrice::new_int(2_100),
                    post_only: true,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    // Query the resting order to get its ID.
    let orders: BTreeMap<perps::OrderId, perps::QueryOrdersByUserResponseItem> = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();

    let order_id = *orders.keys().next().unwrap();

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::CancelOrder(
                perps::CancelOrderRequest::One(order_id),
            )),
            Coins::new(),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish().await?;

    // -----------------------------------------------------------------------
    // Query user2's events in ascending order (oldest first).
    // -----------------------------------------------------------------------
    let user2_addr = accounts.user2.address().to_string();

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                // Ascending order → chronological.
                let response = call_graphql_query::<_, perps_events::ResponseData>(
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

                // user2 lifecycle (ASC):
                //  0: order_persisted  — limit ask at 2000, size -3
                //  1: order_filled     — maker fill at 2000
                //  2: order_removed    — reason Filled (order fully consumed)
                //  3: order_persisted  — second limit ask at 2100, size -2
                //  4: order_removed    — reason Canceled
                let types: Vec<&str> = nodes.iter().map(|n| n.event_type.as_str()).collect();
                assert_eq!(
                    types,
                    &[
                        "order_persisted",
                        "order_filled",
                        "order_removed",
                        "order_persisted",
                        "order_removed",
                    ],
                    "Unexpected event sequence (ASC): {types:?}"
                );

                // [0] order_persisted — first limit ask
                assert!(
                    nodes[0].data.to_string().contains("2000"),
                    "First persist should reference price 2000, got: {}",
                    nodes[0].data
                );

                // [1] order_filled — maker side of the match
                assert!(
                    nodes[1].data.to_string().contains("2000"),
                    "Fill should reference price 2000, got: {}",
                    nodes[1].data
                );
                assert!(
                    nodes[1].data.get("fill_size").is_some(),
                    "Fill event should contain fill_size field"
                );

                // [2] order_removed — fully filled
                assert_eq!(
                    nodes[2].data["reason"].as_str().unwrap(),
                    "filled",
                    "First removal should be reason=filled"
                );

                // [3] order_persisted — second limit ask at 2100
                assert!(
                    nodes[3].data.to_string().contains("2100"),
                    "Second persist should reference price 2100, got: {}",
                    nodes[3].data
                );

                // [4] order_removed — user canceled
                assert_eq!(
                    nodes[4].data["reason"].as_str().unwrap(),
                    "canceled",
                    "Second removal should be reason=canceled"
                );

                // Ascending block_height order.
                for window in nodes.windows(2) {
                    assert!(
                        window[0].block_height <= window[1].block_height,
                        "Events should be in ascending block_height order: {} > {}",
                        window[0].block_height,
                        window[1].block_height,
                    );
                }

                // All events belong to user2 and the correct pair.
                for node in &nodes {
                    assert_that!(node.user_addr.as_str()).is_equal_to(user2_addr.as_str());
                    assert_that!(node.pair_id.as_str()).is_equal_to(pair.to_string().as_str());
                }

                // ---------------------------------------------------------------
                // Query again in descending order — mirror of ascending.
                // ---------------------------------------------------------------
                let response_desc = call_graphql_query::<_, perps_events::ResponseData>(
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
                assert_that!(nodes_desc).has_length(5);

                // DESC is the reverse lifecycle: most recent first.
                //  0: order_removed    — reason Canceled
                //  1: order_persisted  — second limit ask at 2100
                //  2: order_removed    — reason Filled
                //  3: order_filled     — maker fill at 2000
                //  4: order_persisted  — first limit ask at 2000

                // [0] order_removed — cancel (most recent)
                assert_eq!(nodes_desc[0].event_type, "order_removed");
                assert_eq!(nodes_desc[0].data["reason"].as_str().unwrap(), "canceled");

                // [1] order_persisted — second limit ask at 2100
                assert_eq!(nodes_desc[1].event_type, "order_persisted");
                assert!(
                    nodes_desc[1].data.to_string().contains("2100"),
                    "Second persist (DESC[1]) should reference price 2100, got: {}",
                    nodes_desc[1].data
                );

                // [2] order_removed — fully filled
                assert_eq!(nodes_desc[2].event_type, "order_removed");
                assert_eq!(nodes_desc[2].data["reason"].as_str().unwrap(), "filled");

                // [3] order_filled — maker fill at 2000
                assert_eq!(nodes_desc[3].event_type, "order_filled");
                assert!(
                    nodes_desc[3].data.to_string().contains("2000"),
                    "Fill (DESC[3]) should reference price 2000, got: {}",
                    nodes_desc[3].data
                );
                assert!(
                    nodes_desc[3].data.get("fill_size").is_some(),
                    "Fill event should contain fill_size field"
                );

                // [4] order_persisted — first limit ask at 2000 (oldest)
                assert_eq!(nodes_desc[4].event_type, "order_persisted");
                assert!(
                    nodes_desc[4].data.to_string().contains("2000"),
                    "First persist (DESC[4]) should reference price 2000, got: {}",
                    nodes_desc[4].data
                );

                // Descending block_height order.
                for window in nodes_desc.windows(2) {
                    assert!(
                        window[0].block_height >= window[1].block_height,
                        "Events should be in descending block_height order: {} < {}",
                        window[0].block_height,
                        window[1].block_height,
                    );
                }

                // ---------------------------------------------------------------
                // Pagination: fetch first 2, then next page.
                // ---------------------------------------------------------------
                let page1 = call_graphql_query::<_, perps_events::ResponseData>(
                    dango_httpd_context.clone(),
                    PerpsEvents::build_query(perps_events::Variables {
                        user_addr: Some(user2_addr.clone()),
                        sort_by: Some(perps_events::PerpsEventSortBy::BLOCK_HEIGHT_ASC),
                        first: Some(2),
                        ..Default::default()
                    }),
                )
                .await?;

                let page1_data = page1.data.unwrap().perps_events;
                assert_that!(page1_data.nodes).has_length(2);
                assert!(page1_data.page_info.has_next_page, "Should have next page");

                let cursor = page1_data.page_info.end_cursor.unwrap();

                let page2 = call_graphql_query::<_, perps_events::ResponseData>(
                    dango_httpd_context.clone(),
                    PerpsEvents::build_query(perps_events::Variables {
                        user_addr: Some(user2_addr.clone()),
                        sort_by: Some(perps_events::PerpsEventSortBy::BLOCK_HEIGHT_ASC),
                        first: Some(2),
                        after: Some(cursor),
                        ..Default::default()
                    }),
                )
                .await?;

                let page2_data = page2.data.unwrap().perps_events;
                assert_that!(page2_data.nodes.len()).is_at_least(1);

                // Page 2 first event must come after page 1 last event.
                assert!(
                    page2_data.nodes[0].block_height
                        >= page1_data.nodes.last().unwrap().block_height,
                    "Page 2 events should come after page 1"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
