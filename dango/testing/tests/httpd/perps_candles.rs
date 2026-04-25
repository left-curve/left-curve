use {
    crate::{build_actix_app, call_graphql_query},
    assertor::*,
    dango_indexer_clickhouse::indexer::perps_candles::cache::PerpsCandleCache,
    dango_sdk::{PerpsCandles, SubscribePerpsCandles, perps_candles, subscribe_perps_candles},
    dango_testing::{
        TestOption,
        perps::{create_perps_fill, pair_id, setup_perps_env},
        setup_test_with_indexer,
    },
    graphql_client::GraphQLQuery,
    grug_app::Indexer,
    indexer_testing::{
        GraphQLCustomRequest, call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    std::{collections::HashMap, sync::Arc},
    tokio::sync::{Mutex, mpsc},
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn query_perps_candles() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);

    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair_id(), 2_000, 5);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = perps_candles::Variables {
                    pair_id: pair_id().to_string(),
                    interval: perps_candles::CandleInterval::ONE_MINUTE,
                    ..Default::default()
                };

                let response = call_graphql_query::<_, perps_candles::ResponseData>(
                    dango_httpd_context.clone(),
                    PerpsCandles::build_query(variables),
                )
                .await?;

                let data = response.data.unwrap();
                let nodes = data.perps_candles.nodes;

                assert_that!(nodes.len()).is_equal_to(1);

                let candle = &nodes[0];
                assert_that!(candle.pair_id.as_str()).is_equal_to("perp/ethusd");
                assert_that!(candle.open.as_str()).is_equal_to("2000");
                assert_that!(candle.high.as_str()).is_equal_to("2000");
                assert_that!(candle.low.as_str()).is_equal_to("2000");
                assert_that!(candle.close.as_str()).is_equal_to("2000");
                assert_that!(candle.interval)
                    .is_equal_to(perps_candles::CandleInterval::ONE_MINUTE);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_perps_candles() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);

    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair_id(), 2_000, 5);
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair_id(), 2_000, 5);

    suite.app.indexer.wait_for_finish().await?;

    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribePerpsCandles::build_query(subscribe_perps_candles::Variables {
            pair_id: pair_id().to_string(),
            interval: subscribe_perps_candles::CandleInterval::ONE_MINUTE,
            later_than: None,
        }),
        "perpsCandles",
    );

    let local_set = tokio::task::LocalSet::new();
    let suite = Arc::new(Mutex::new(suite));
    let suite_clone = suite.clone();

    let (create_candle_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let mut suite_guard = suite_clone.lock().await;
            create_perps_fill(
                &mut suite_guard,
                &mut accounts,
                &contracts,
                &pair_id(),
                2_000,
                1,
            );
        }
        Ok::<(), anyhow::Error>(())
    });

    let create_candle_tx_clone = create_candle_tx.clone();
    let context = dango_httpd_context.clone();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response: existing last candle
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_perps_candles::SubscribePerpsCandlesPerpsCandles>,
                >(&mut framed, name)
                .await?;

                assert!(!response.data.is_empty(), "Expected at least one candle");
                let candle = &response.data[0];
                assert_eq!(candle.pair_id, "perp/ethusd");
                assert_eq!(candle.open, "2000");

                let initial_max_block = candle.max_block_height;

                // Trigger a new fill
                create_candle_tx_clone.send(1).await.unwrap();

                // 2nd response: updated candle
                loop {
                    let response = parse_graphql_subscription_response::<
                        Vec<subscribe_perps_candles::SubscribePerpsCandlesPerpsCandles>,
                    >(&mut framed, name)
                    .await?;

                    if response.data.first().unwrap().max_block_height <= initial_max_block {
                        continue;
                    }

                    assert!(!response.data.is_empty());
                    let candle = &response.data[0];
                    assert_eq!(candle.pair_id, "perp/ethusd");
                    assert!(
                        candle.max_block_height > initial_max_block,
                        "Block height should have increased"
                    );

                    break;
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    {
        let suite_guard = suite.lock().await;
        suite_guard.app.indexer.wait_for_finish().await.unwrap();
    }

    // Verify cache consistency after subscription
    let mut fresh_cache = PerpsCandleCache::default();
    let pair_ids =
        dango_indexer_clickhouse::entities::perps_pair_price::PerpsPairPrice::all_pair_ids(
            context.indexer_clickhouse_context.clickhouse_client(),
        )
        .await?;

    fresh_cache
        .preload_pairs(
            &pair_ids,
            context.indexer_clickhouse_context.clickhouse_client(),
        )
        .await?;

    let old_cache = context
        .indexer_clickhouse_context
        .perps_candle_cache
        .read()
        .await;

    assert_eq!(
        fresh_cache.pair_prices,
        old_cache
            .pair_prices
            .clone()
            .into_iter()
            .filter(|pp| !pp.1.is_empty())
            .collect::<HashMap<_, _>>()
    );
    assert_eq!(fresh_cache.candles, old_cache.candles);

    drop(old_cache);

    let mut suite_guard = suite.lock().await;
    suite_guard
        .app
        .indexer
        .shutdown()
        .await
        .expect("Can't shutdown indexer");

    Ok(())
}
