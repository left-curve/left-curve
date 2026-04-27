use {
    crate::call_graphql_query,
    assertor::*,
    dango_sdk::{PerpsEvents, perps_events},
    dango_testing::{
        TestOption,
        perps::{create_perps_fill, pair_id, setup_perps_env},
        setup_test_with_indexer,
    },
    graphql_client::GraphQLQuery,
    grug::Addressable,
    grug_app::Indexer,
};

/// Query a user's perps events and verify that only `order_filled` events
/// are indexed after a limit-order match.
#[tokio::test(flavor = "multi_thread")]
async fn query_perps_events_user_lifecycle() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);

    // user2 places a limit ask → user1 fills it via market buy.
    //   user2 sees: order_filled
    //   user1 sees: order_filled
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 3);

    suite.app.indexer.wait_for_finish().await?;

    let user2_addr = accounts.user2.address().to_string();

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                // Ascending order.
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
                assert_that!(nodes_desc).has_length(1);
                assert_eq!(nodes_desc[0].event_type, "order_filled");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
