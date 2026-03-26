use {
    crate::call_graphql_query,
    dango_testing::{
        TestOption,
        perps::{create_perps_fill, pair_id, setup_perps_env},
        setup_test_with_indexer,
    },
    dango_types::{Dimensionless, Quantity, UsdPrice, perps},
    graphql_client::GraphQLQuery,
    grug::{Coins, Dec128_6, ResultExt},
    grug_app::Indexer,
    indexer_client::{
        AllPerpsPairStats, PerpsPairStats, PerpsPairStatsPartial, all_perps_pair_stats,
        perps_pair_stats, perps_pair_stats_partial,
    },
    std::str::FromStr,
};

#[tokio::test(flavor = "multi_thread")]
async fn query_perps_pair_stats() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 5);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, perps_pair_stats::ResponseData>(
                    dango_httpd_context.clone(),
                    PerpsPairStats::build_query(perps_pair_stats::Variables {
                        pair_id: pair.to_string(),
                    }),
                )
                .await?;

                let data = response.data.expect("Expected perpsPairStats response data");
                let stats = data
                    .perps_pair_stats
                    .expect("Expected perps pair stats to be returned");

                assert_eq!(stats.pair_id, "perp/ethusd");
                assert!(stats.current_price.is_some(), "Expected current_price");
                assert!(stats.price24_h_ago.is_some(), "Expected price24_h_ago");

                // Since we just created the fill, current_price and price_24h_ago
                // should be the same (no price from 24h ago, so earliest price is used)
                assert_eq!(
                    stats.current_price, stats.price24_h_ago,
                    "With fresh data, current and 24h ago prices should match"
                );

                // Price change should be 0 since prices are the same
                if let Some(price_change) = &stats.price_change24_h {
                    assert_eq!(
                        price_change, "0",
                        "Expected 0 price change, got {price_change}"
                    );
                }

                // Volume should be non-zero since we created a fill
                assert_ne!(
                    stats.volume24_h, "0",
                    "Expected non-zero 24h volume after fill"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_perps_pair_stats_nonexistent_pair() -> anyhow::Result<()> {
    let (suite, _, _, _, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response =
                    call_graphql_query::<_, perps_pair_stats_partial::ResponseData>(
                        dango_httpd_context.clone(),
                        PerpsPairStatsPartial::build_query(perps_pair_stats_partial::Variables {
                            pair_id: "perp/nonexistent".to_string(),
                        }),
                    )
                    .await?;

                let data = response
                    .data
                    .expect("Expected perpsPairStats partial response data");

                assert!(
                    data.perps_pair_stats.is_none(),
                    "Expected None for nonexistent pair, got {:?}",
                    data.perps_pair_stats
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_all_perps_pair_stats() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 5);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, all_perps_pair_stats::ResponseData>(
                    dango_httpd_context.clone(),
                    AllPerpsPairStats::build_query(all_perps_pair_stats::Variables),
                )
                .await?;

                let data = response
                    .data
                    .expect("Expected allPerpsPairStats response data");
                let all_stats = data.all_perps_pair_stats;

                assert!(
                    !all_stats.is_empty(),
                    "Expected at least one pair in allPerpsPairStats"
                );

                // Find the perp/ethusd pair
                let eth_stats = all_stats.iter().find(|s| s.pair_id == "perp/ethusd");

                assert!(
                    eth_stats.is_some(),
                    "Expected perp/ethusd pair in allPerpsPairStats"
                );

                let stats = eth_stats.unwrap();
                assert!(stats.current_price.is_some(), "Expected current_price");
                assert!(stats.price24_h_ago.is_some(), "Expected price24_h_ago");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_perps_pair_stats_partial_fields() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);
    create_perps_fill(&mut suite, &mut accounts, &contracts, &pair, 2_000, 5);

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response =
                    call_graphql_query::<_, perps_pair_stats_partial::ResponseData>(
                        dango_httpd_context.clone(),
                        PerpsPairStatsPartial::build_query(perps_pair_stats_partial::Variables {
                            pair_id: pair.to_string(),
                        }),
                    )
                    .await?;

                let data = response
                    .data
                    .expect("Expected perpsPairStats partial response data");
                let stats = data
                    .perps_pair_stats
                    .expect("Expected perps pair stats to be returned");

                assert_eq!(stats.pair_id, "perp/ethusd");
                assert!(
                    stats.current_price.is_some(),
                    "Expected current_price to be present"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_perps_pair_stats_formats_small_prices_without_scientific_notation(
) -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    let pair = pair_id();
    // Use a high oracle price so the small fill price doesn't violate margin checks.
    setup_perps_env(&mut suite, &mut accounts, &contracts, 2_000, 100_000);

    // Submit matching limit ask + market buy at a tiny price so GraphQL must
    // serialize small decimals without scientific notation.
    let tiny_price =
        UsdPrice::new(Dec128_6::from_str("0.001").expect("tiny fixed-point price should parse"));

    suite
        .execute(
            &mut accounts.user2,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(-5),
                kind: perps::OrderKind::Limit {
                    limit_price: tiny_price,
                    post_only: true,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    suite
        .execute(
            &mut accounts.user1,
            contracts.perps,
            &perps::ExecuteMsg::Trade(perps::TraderMsg::SubmitOrder {
                pair_id: pair.clone(),
                size: Quantity::new_int(5),
                kind: perps::OrderKind::Market {
                    max_slippage: Dimensionless::ONE,
                },
                reduce_only: false,
            }),
            Coins::new(),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, perps_pair_stats::ResponseData>(
                    dango_httpd_context.clone(),
                    PerpsPairStats::build_query(perps_pair_stats::Variables {
                        pair_id: pair.to_string(),
                    }),
                )
                .await?;

                let data = response.data.expect("Expected perpsPairStats response data");
                let stats = data
                    .perps_pair_stats
                    .expect("Expected perps pair stats to be returned");

                let current_price = stats.current_price.expect("Expected current_price");
                let price_24h_ago = stats.price24_h_ago.expect("Expected price24_h_ago");

                assert_eq!(current_price, "0.001");
                assert_eq!(price_24h_ago, "0.001");

                assert!(
                    !current_price.contains('e') && !current_price.contains('E'),
                    "current_price should use plain decimal notation, got: {current_price}"
                );
                assert!(
                    !price_24h_ago.contains('e') && !price_24h_ago.contains('E'),
                    "price_24h_ago should use plain decimal notation, got: {price_24h_ago}"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
