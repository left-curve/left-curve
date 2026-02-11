use {
    crate::call_graphql_query,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    graphql_client::GraphQLQuery,
    grug::{
        Coin, Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, ResultExt, Signer,
        StdResult, Timestamp, Udec128, Udec128_24, Uint128, btree_map,
    },
    grug_app::Indexer,
    indexer_client::{
        AllPairStats, PairStats, PairStatsPartial, all_pair_stats, pair_stats, pair_stats_partial,
    },
};

#[tokio::test(flavor = "multi_thread")]
async fn query_pair_stats() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, pair_stats::ResponseData>(
                    dango_httpd_context.clone(),
                    PairStats::build_query(pair_stats::Variables {
                        base_denom: "dango".to_string(),
                        quote_denom: "bridge/usdc".to_string(),
                    }),
                )
                .await?;

                let data = response.data.expect("Expected pairStats response data");
                let response = data.pair_stats.expect("Expected pair stats to be returned");

                assert_eq!(response.base_denom, "dango");
                assert_eq!(response.quote_denom, "bridge/usdc");
                assert!(response.current_price.is_some(), "Expected current_price");
                assert!(response.price24_h_ago.is_some(), "Expected price24_h_ago");

                // Since we just created the pair prices, current_price and price_24h_ago
                // should be the same (no price from 24h ago, so earliest price is used)
                assert_eq!(
                    response.current_price, response.price24_h_ago,
                    "With fresh data, current and 24h ago prices should match"
                );

                // Price change should be 0 since prices are the same
                if let Some(price_change) = &response.price_change24_h {
                    assert_eq!(
                        price_change, "0",
                        "Expected 0 price change, got {price_change}"
                    );
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_pair_stats_nonexistent_pair() -> anyhow::Result<()> {
    let (suite, _, _, _, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, pair_stats_partial::ResponseData>(
                    dango_httpd_context.clone(),
                    PairStatsPartial::build_query(pair_stats_partial::Variables {
                        base_denom: "nonexistent".to_string(),
                        quote_denom: "fake/token".to_string(),
                    }),
                )
                .await?;

                let data = response
                    .data
                    .expect("Expected pairStats partial response data");

                assert!(
                    data.pair_stats.is_none(),
                    "Expected None for nonexistent pair, got {:?}",
                    data.pair_stats
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_all_pair_stats() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, all_pair_stats::ResponseData>(
                    dango_httpd_context.clone(),
                    AllPairStats::build_query(all_pair_stats::Variables),
                )
                .await?;

                let data = response.data.expect("Expected allPairStats response data");
                let response = data.all_pair_stats;

                assert!(
                    !response.is_empty(),
                    "Expected at least one pair in allPairStats"
                );

                // Find the dango/usdc pair
                let dango_usdc = response
                    .iter()
                    .find(|p| p.base_denom == "dango" && p.quote_denom == "bridge/usdc");

                assert!(
                    dango_usdc.is_some(),
                    "Expected dango/usdc pair in allPairStats"
                );

                let pair = dango_usdc.unwrap();
                assert!(pair.current_price.is_some(), "Expected current_price");
                assert!(pair.price24_h_ago.is_some(), "Expected price24_h_ago");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_pair_stats_partial_fields() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response = call_graphql_query::<_, pair_stats_partial::ResponseData>(
                    dango_httpd_context.clone(),
                    PairStatsPartial::build_query(pair_stats_partial::Variables {
                        base_denom: "dango".to_string(),
                        quote_denom: "bridge/usdc".to_string(),
                    }),
                )
                .await?;

                let data = response
                    .data
                    .expect("Expected pairStats partial response data");
                let response = data.pair_stats.expect("Expected pair stats to be returned");

                assert_eq!(response.base_denom, "dango");
                assert_eq!(response.quote_denom, "bridge/usdc");
                assert!(
                    response.current_price.is_some(),
                    "Expected current_price to be present"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

async fn create_pair_prices(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> anyhow::Result<()> {
    suite
        .execute(
            &mut accounts.owner,
            contracts.oracle,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                dango::DENOM.clone() => PriceSource::Fixed {
                    humanized_price: Udec128::ONE,
                    precision: 6,
                    timestamp: Timestamp::from_seconds(1730802926),
                },
            }),
            Coins::new(),
        )
        .should_succeed();

    let orders_to_submit: Vec<(Direction, u128, u128)> = vec![
        (Direction::Bid, 30, 25), // !0 - filled
        (Direction::Bid, 20, 10), // !1 - unfilled
        (Direction::Bid, 10, 10), // !2 - unfilled
        (Direction::Ask, 5, 10),  //  3 - filled
        (Direction::Ask, 15, 10), //  4 - filled
        (Direction::Ask, 25, 10), //  5 - 50% filled
    ];

    // Submit the orders in a single block.
    let txs = orders_to_submit
        .into_iter()
        .zip(accounts.users_mut())
        .map(|((direction, price, amount), signer)| {
            let price = Udec128_24::new(price);
            let amount = Uint128::new(amount);

            let fund = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coin::new(usdc::DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coin::new(dango::DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates: vec![CreateOrderRequest::new_limit(
                        dango::DENOM.clone(),
                        usdc::DENOM.clone(),
                        direction,
                        NonZero::new_unchecked(price),
                        NonZero::new_unchecked(fund.amount),
                    )],
                    cancels: None,
                },
                Coins::from(fund),
            )?;

            signer.sign_transaction(NonEmpty::new_unchecked(vec![msg]), &suite.chain_id, 100_000)
        })
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    // Make a block with the order submissions. Ensure all transactions were
    // successful.
    suite
        .make_block(txs)
        .block_outcome
        .tx_outcomes
        .into_iter()
        .for_each(|outcome| {
            outcome.should_succeed();
        });

    Ok(())
}
