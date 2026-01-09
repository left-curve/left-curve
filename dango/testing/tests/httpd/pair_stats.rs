use {
    crate::build_actix_app,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        Coin, Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, ResultExt, Signer,
        StdResult, Timestamp, Udec128, Udec128_24, Uint128, btree_map,
    },
    grug_app::Indexer,
    indexer_testing::{GraphQLCustomRequest, call_graphql},
    serde::Deserialize,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PairStatsResponse {
    base_denom: String,
    quote_denom: String,
    current_price: Option<String>,
    #[serde(rename = "price24HAgo")]
    price_24h_ago: Option<String>,
    #[serde(rename = "volume24H")]
    volume_24h: String,
    #[serde(rename = "priceChange24H")]
    price_change_24h: Option<String>,
}

#[tokio::test(flavor = "multi_thread")]
async fn query_pair_stats() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let graphql_query = r#"
        query PairStats($base_denom: String!, $quote_denom: String!) {
            pairStats(baseDenom: $base_denom, quoteDenom: $quote_denom) {
                baseDenom
                quoteDenom
                currentPrice
                price24HAgo
                volume24H
                priceChange24H
            }
        }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "pairStats",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "dango",
            "quote_denom": "bridge/usdc",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context.clone());

                let response: Option<PairStatsResponse> =
                    call_graphql(app, request_body.clone()).await?.data;

                let response = response.expect("Expected pair stats to be returned");

                assert_eq!(response.base_denom, "dango");
                assert_eq!(response.quote_denom, "bridge/usdc");
                assert!(response.current_price.is_some(), "Expected current_price");
                assert!(response.price_24h_ago.is_some(), "Expected price_24h_ago");

                // Since we just created the pair prices, current_price and price_24h_ago
                // should be the same (no price from 24h ago, so earliest price is used)
                assert_eq!(
                    response.current_price, response.price_24h_ago,
                    "With fresh data, current and 24h ago prices should match"
                );

                // Price change should be 0% since prices are the same
                if let Some(price_change) = &response.price_change_24h {
                    let change: f64 = price_change.parse().unwrap();
                    assert!(
                        change.abs() < 0.0001,
                        "Expected ~0% price change, got {change}"
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

    let graphql_query = r#"
        query PairStats($base_denom: String!, $quote_denom: String!) {
            pairStats(baseDenom: $base_denom, quoteDenom: $quote_denom) {
                baseDenom
                quoteDenom
                currentPrice
            }
        }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "pairStats",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "nonexistent",
            "quote_denom": "fake/token",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context.clone());

                let response: Option<PairStatsResponse> =
                    call_graphql(app, request_body.clone()).await?.data;

                assert!(
                    response.is_none(),
                    "Expected None for nonexistent pair, got {response:?}"
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

    let graphql_query = r#"
        query AllPairStats {
            allPairStats {
                baseDenom
                quoteDenom
                currentPrice
                price24HAgo
                volume24H
                priceChange24H
            }
        }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "allPairStats",
        query: graphql_query,
        variables: serde_json::Map::new(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context.clone());

                let response: Vec<PairStatsResponse> =
                    call_graphql(app, request_body.clone()).await?.data;

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
                assert!(pair.price_24h_ago.is_some(), "Expected price_24h_ago");

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

    // Query only currentPrice - this should only trigger one DB query
    let graphql_query = r#"
        query PairStats($base_denom: String!, $quote_denom: String!) {
            pairStats(baseDenom: $base_denom, quoteDenom: $quote_denom) {
                baseDenom
                quoteDenom
                currentPrice
            }
        }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "pairStats",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "dango",
            "quote_denom": "bridge/usdc",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context.clone());

                #[derive(Deserialize, Debug)]
                #[serde(rename_all = "camelCase")]
                struct PartialPairStats {
                    base_denom: String,
                    quote_denom: String,
                    current_price: Option<String>,
                }

                let response: Option<PartialPairStats> =
                    call_graphql(app, request_body.clone()).await?.data;

                let response = response.expect("Expected pair stats to be returned");

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
