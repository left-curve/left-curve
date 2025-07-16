use {
    crate::build_actix_app,
    assertor::*,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, ResultExt, Signer,
        StdResult, Timestamp, Udec128, Uint128, btree_map, setup_tracing_subscriber,
    },
    grug_app::Indexer,
    indexer_clickhouse::entities::{candle::Candle, pair_price::PairPrice},
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse, call_paginated_graphql, call_ws_graphql_stream,
        parse_graphql_subscription_response,
    },
    tokio::sync::mpsc,
    tracing::Level,
};

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_candles() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);

    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(true).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    let graphql_query = r#"
      query Candles($base_denom: String!, $quote_denom: String!, $interval: String) {
      candles(baseDenom: $base_denom, quoteDenom: $quote_denom, interval: $interval) {
          nodes {
            timeStart
            open
            high
            low
            close
            volumeBase
            volumeQuote
            quoteDenom
            baseDenom
            interval
          }
          edges { node { timeStart open high low close volumeBase volumeQuote interval baseDenom quoteDenom }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "candles",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "dango",
            "quote_denom": "bridge/usdc",
            "interval": "ONE_SECOND",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context);

                let response: PaginatedResponse<serde_json::Value> =
                    call_paginated_graphql(app, request_body).await?;

                let received_candles = response
                    .edges
                    .into_iter()
                    .map(|e| e.node)
                    .collect::<Vec<_>>();

                let expected_candle = serde_json::json!({
                    "timeStart": "1971-01-01T00:00:00Z",
                    "open": "27.4",
                    "high": "27.4",
                    "low": "27.4",
                    "close": "27.4",
                    "volumeBase": "25",
                    "volumeQuote": "718",
                    "interval": "ONE_SECOND",
                    "baseDenom": "dango",
                    "quoteDenom": "bridge/usdc",
                });

                assert_that!(received_candles).is_equal_to(vec![expected_candle]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_candles() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);

    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, clickhouse_context) =
        setup_test_with_indexer(true).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;
    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
    subscription Candles($base_denom: String!, $quote_denom: String!, $interval: String, $later_than: String) {
        candles(baseDenom: $base_denom, quoteDenom: $quote_denom, interval: $interval, laterThan: $later_than) {
            timeStart
            open
            high
            low
            close
            volumeBase
            volumeQuote
            quoteDenom
            baseDenom
            interval
        }
    }
  "#;

    let request_body = GraphQLCustomRequest {
        name: "candles",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "dango",
            "quote_denom": "bridge/usdc",
            "interval": "ONE_SECOND",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    // Can't call this from LocalSet so using channels instead.
    let (create_candle_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(block_height) = rx.recv().await {
            if block_height == 0 {
                break;
            }
            tracing::info!("Creating pair prices");

            create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

            tracing::info!("Pair prices created");
            // Enabling this here will cause the test to hang
            suite.app.indexer.wait_for_finish()?;
        }

        tracing::info!("dropping suite and rx");
        drop(suite);
        drop(rx);
        tracing::info!("dropped suite and rx");
        Ok::<(), anyhow::Error>(())
    });

    let create_candle_tx_clone = create_candle_tx.clone();
    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last candle
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<serde_json::Value>>(framed, name)
                        .await?;

                println!("response: {response:#?}");

                // assert_that!(
                //     response
                //         .data
                //         .into_iter()
                //         .map(|t| t.created_block_height)
                //         .collect::<Vec<_>>()
                // )
                // .is_equal_to(vec![2]);

                tracing::info!("sending 2");

                create_candle_tx_clone.send(2).await.unwrap();

                // // 2nd response
                let (_, response) =
                    parse_graphql_subscription_response::<Vec<serde_json::Value>>(framed, name)
                        .await?;

                println!("response: {response:#?}");

                // assert_that!(
                //     response
                //         .data
                //         .into_iter()
                //         .map(|t| t.created_block_height)
                //         .collect::<Vec<_>>()
                // )
                // .is_equal_to(vec![4]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    tracing::info!("finished local set");

    let candle_1s: Vec<Candle> = clickhouse_context
        .clickhouse_client()
        .query("SELECT *, '1s' as interval FROM pair_prices_1s")
        .fetch_all()
        .await?;

    println!("candle_1s: {:#?}", candle_1s.len());

    let candle_1m: Vec<Candle> = clickhouse_context
        .clickhouse_client()
        .query("SELECT *, '1m' as interval FROM pair_prices_1m")
        .fetch_all()
        .await?;

    println!("candle_1m: {:#?}", candle_1m.len());

    let pair_prices: Vec<PairPrice> = clickhouse_context
        .clickhouse_client()
        .query("SELECT * FROM pair_prices")
        .fetch_all()
        .await?;

    println!("pair_prices: {:#?}", pair_prices.len());

    create_candle_tx.send(0).await.unwrap();

    tracing::info!("finished test");

    Ok(())
}

async fn create_pair_prices(
    suite: &mut TestSuiteWithIndexer,
    accounts: &mut TestAccounts,
    contracts: &Contracts,
) -> anyhow::Result<()> {
    tracing::info!("create_pair_prices called");

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
            let price = Udec128::new(price);
            let amount = Uint128::new(amount);

            let funds = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    Coins::one(usdc::DENOM.clone(), quote_amount).unwrap()
                },
                Direction::Ask => Coins::one(dango::DENOM.clone(), amount).unwrap(),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![CreateLimitOrderRequest {
                        base_denom: dango::DENOM.clone(),
                        quote_denom: usdc::DENOM.clone(),
                        direction,
                        amount: NonZero::new_unchecked(amount),
                        price,
                    }],
                    cancels: None,
                },
                funds,
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

    tracing::info!("create_pair_prices finished");

    Ok(())
}
