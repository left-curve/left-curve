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
    let orders: perps::QueryOrdersByUserResponse = suite
        .query_wasm_smart(contracts.perps, perps::QueryOrdersByUserRequest {
            user: accounts.user2.address(),
        })
        .should_succeed();

    let order_id = orders.asks[0].order_id;

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

                // user2 should see: order_persisted, order_filled, order_persisted, order_removed
                assert_that!(nodes.len()).is_at_least(4);

                let types: Vec<&str> = nodes.iter().map(|n| n.event_type.as_str()).collect();

                // First event must be order_persisted (the limit ask placement).
                assert_that!(types[0]).is_equal_to("order_persisted");

                // Must contain a fill event.
                assert!(
                    types.contains(&"order_filled"),
                    "Expected order_filled in events, got: {types:?}"
                );

                // The last two events should be the second persist + cancel.
                let last_two: Vec<&str> = types.iter().rev().take(2).copied().collect();
                assert_that!(last_two).contains("order_persisted");
                assert_that!(last_two).contains("order_removed");

                // Verify ascending block_height order.
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
                // Query again in descending order (default) — should be reversed.
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

                assert_that!(nodes_desc.len()).is_equal_to(nodes.len() as usize);

                // Verify descending block_height order.
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
