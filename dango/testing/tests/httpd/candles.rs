use {
    crate::build_actix_app,
    assert_json_diff::assert_json_include,
    dango_genesis::Contracts,
    dango_indexer_clickhouse::{
        entities::pair_price::PairPrice, indexer::candles::cache::CandleCache,
    },
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateLimitOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    grug::{
        Addressable, Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst, ResultExt,
        Signer, StdResult, Timestamp, Udec128, Udec128_24, Uint128, btree_map,
    },
    grug_app::Indexer,
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse, call_paginated_graphql, call_ws_graphql_stream,
        parse_graphql_subscription_response,
    },
    std::{collections::HashMap, sync::Arc},
    tokio::sync::{Mutex, mpsc},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_candles() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

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
            blockHeight
          }
          edges { node { timeStart open high low close volumeBase volumeQuote interval baseDenom quoteDenom blockHeight }  cursor }
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
                let mut received_candles: Vec<serde_json::Value> = vec![];

                for _ in 0..10 {
                    let app = build_actix_app(dango_httpd_context.clone());

                    let response: PaginatedResponse<serde_json::Value> =
                        call_paginated_graphql(app, request_body.clone()).await?;

                    received_candles = response
                        .edges
                        .into_iter()
                        .map(|e| e.node)
                        .collect::<Vec<_>>();
                }

                let expected_candle = serde_json::json!({
                    "timeStart": "1971-01-01T00:00:00Z",
                    "open": "27.5",
                    "high": "27.5",
                    "low": "27.5",
                    "close": "27.5",
                    "volumeBase": "25",
                    "volumeQuote": "687.5",
                    "interval": "ONE_SECOND",
                    "baseDenom": "dango",
                    "quoteDenom": "bridge/usdc",
                });

                assert_json_include!(actual: received_candles, expected: [expected_candle]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_candles_with_dates() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Candles($base_denom: String!, $quote_denom: String!, $interval: String, $earlierThan: DateTime) {
      candles(baseDenom: $base_denom, quoteDenom: $quote_denom, interval: $interval, earlierThan: $earlierThan) {
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
            blockHeight
          }
          edges { node { timeStart open high low close volumeBase volumeQuote interval baseDenom quoteDenom blockHeight }  cursor }
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
            "earlierThan": "2025-07-24T07:00:00.000Z",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut received_candles: Vec<serde_json::Value> = vec![];

                for _ in 0..10 {
                    let app = build_actix_app(dango_httpd_context.clone());

                    let response: PaginatedResponse<serde_json::Value> =
                        call_paginated_graphql(app, request_body.clone()).await?;

                    received_candles = response
                        .edges
                        .into_iter()
                        .map(|e| e.node)
                        .collect::<Vec<_>>();
                }

                let expected_candle = serde_json::json!({
                    "timeStart": "1971-01-01T00:00:00Z",
                    "open": "27.5",
                    "high": "27.5",
                    "low": "27.5",
                    "close": "27.5",
                    "volumeBase": "25",
                    "volumeQuote": "687.5",
                    "interval": "ONE_SECOND",
                    "baseDenom": "dango",
                    "quoteDenom": "bridge/usdc",
                });

                assert_json_include!(actual: received_candles, expected: [expected_candle]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_candles() -> anyhow::Result<()> {
    let _span = tracing::info_span!("graphql_subscribe_to_candles").entered();

    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

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
              blockHeight
          }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "candles",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "dango",
            "quote_denom": "bridge/usdc",
            "interval": "ONE_MINUTE",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();
    let suite = Arc::new(Mutex::new(suite));
    let suite_clone = suite.clone();

    // Can't call this from LocalSet so using channels instead.
    let (create_candle_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let mut suite_guard = suite_clone.lock().await;

            create_pair_prices(&mut suite_guard, &mut accounts, &contracts).await?;

            // Enabling this here will cause the test to hang
            // suite_guard.app.indexer.wait_for_finish()?;
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

                // 1st response is always the existing last candle
                let response = parse_graphql_subscription_response::<Vec<serde_json::Value>>(
                    &mut framed,
                    name,
                )
                .await?;

                let expected_json = serde_json::json!([{
                    "baseDenom": "dango",
                    "quoteDenom": "bridge/usdc",
                    "interval": "ONE_MINUTE",
                    "close": "25",
                    "high": "27.5",
                    "low": "25",
                    "open": "27.5",
                    "volumeBase": "50",
                    "volumeQuote": "1312.5",
                    "blockHeight": 4,
                }]);

                assert_json_include!(actual: response.data, expected: expected_json);

                create_candle_tx_clone.send(2).await.unwrap();

                loop {
                    // 2nd response
                    let response = parse_graphql_subscription_response::<Vec<serde_json::Value>>(
                        &mut framed,
                        name,
                    )
                    .await?;

                    // This because blocks aren't indexed in order
                    if response
                        .data
                        .first()
                        .unwrap()
                        .get("blockHeight")
                        .and_then(|v| v.as_u64())
                        .unwrap()
                        < 6
                    {
                        continue;
                    }

                    let expected_json = serde_json::json!([{
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "interval": "ONE_MINUTE",
                        "close": "25",
                        "high": "27.5",
                        "low": "25",
                        "open": "27.5",
                        "blockHeight": 6,
                        "volumeBase": "75",
                        "volumeQuote": "1937.5",
                    }]);

                    assert_json_include!(actual: response.data, expected: expected_json);

                    break;
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    {
        let suite_guard = suite.lock().await;
        suite_guard.app.indexer.wait_for_finish().unwrap();
    }

    // The following ensures than loading clickhouse data to a new cache result in the same
    // loaded data than our current cache
    let mut cache = CandleCache::default();
    let pairs =
        PairPrice::all_pairs(context.indexer_clickhouse_context.clickhouse_client()).await?;

    cache
        .preload_pairs(
            &pairs,
            context.indexer_clickhouse_context.clickhouse_client(),
        )
        .await?;

    let old_cache = context.indexer_clickhouse_context.candle_cache.read().await;

    // println!("Cache : {:#?}", old_cache.pair_prices);
    // println!("Cache from clickhouse: {:#?}", cache.pair_prices);
    assert_eq!(
        cache.pair_prices,
        // Filtering empty ones, since cache has them but not clickhouse
        old_cache
            .pair_prices
            .clone()
            .into_iter()
            .filter(|pp| !pp.1.is_empty())
            .collect::<HashMap<_, _>>()
    );

    // println!("Cache : {:#?}", old_cache.candles);
    // println!("Cache from clickhouse: {:#?}", cache.candles);
    assert_eq!(cache.candles, old_cache.candles);

    drop(old_cache);

    let mut suite_guard = suite.lock().await;

    suite_guard
        .app
        .indexer
        .shutdown()
        .expect("Can't shutdown indexer");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_candles_on_no_new_pair_prices() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

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
              blockHeight
          }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "candles",
        query: graphql_query,
        variables: serde_json::json!({
            "base_denom": "dango",
            "quote_denom": "bridge/usdc",
            "interval": "ONE_MINUTE",
        })
        .as_object()
        .unwrap()
        .clone(),
    };

    let local_set = tokio::task::LocalSet::new();
    let suite = Arc::new(Mutex::new(suite));
    let suite_clone = suite.clone();

    // Can't call this from LocalSet so using channels instead.
    // Creating a block without creating a candle (no new pair prices).
    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while let Some(_idx) = rx.recv().await {
            let msgs = vec![Message::transfer(
                accounts.user2.address(),
                Coins::one(usdc::DENOM.clone(), 123).unwrap(),
            )?];

            let mut suite_guard = suite_clone.lock().await;

            suite_guard
                .send_messages_with_gas(
                    &mut accounts.user1,
                    50_000_000,
                    NonEmpty::new_unchecked(msgs),
                )
                .should_succeed();

            // Enabling this here will cause the test to hang
            // suite_guard.app.indexer.wait_for_finish();
        }

        Ok::<(), anyhow::Error>(())
    });

    let crate_block_tx_clone = crate_block_tx.clone();
    let context = dango_httpd_context.clone();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                // 1st response is always the existing last candle
                let response = parse_graphql_subscription_response::<Vec<serde_json::Value>>(
                    &mut framed,
                    name,
                )
                .await?;

                let expected_json = serde_json::json!([{
                    "baseDenom": "dango",
                    "quoteDenom": "bridge/usdc",
                    "blockHeight": 2,
                    "close": "27.5",
                    "high": "27.5",
                    "interval": "ONE_MINUTE",
                    "low": "27.5",
                    "open": "27.5",
                    "volumeBase": "25",
                    "volumeQuote": "687.5",
                }]);

                assert_json_include!(actual: response.data, expected: expected_json);

                crate_block_tx_clone.send(2).await.unwrap();

                loop {
                    // 2nd response
                    let response = parse_graphql_subscription_response::<Vec<serde_json::Value>>(
                        &mut framed,
                        name,
                    )
                    .await?;

                    // This because blocks aren't indexed in order
                    if response
                        .data
                        .first()
                        .unwrap()
                        .get("blockHeight")
                        .and_then(|v| v.as_u64())
                        .unwrap()
                        < 3
                    {
                        continue;
                    }

                    let expected_json = serde_json::json!([{
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "blockHeight": 3,
                        "close": "27.5",
                        "high": "27.5",
                        "interval": "ONE_MINUTE",
                        "low": "27.5",
                        "open": "27.5",
                        "volumeBase": "25",
                        "volumeQuote": "687.5",
                    }]);

                    assert_json_include!(actual: response.data, expected: expected_json);

                    break;
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    {
        let suite_guard = suite.lock().await;
        suite_guard.app.indexer.wait_for_finish().unwrap();
    }

    // The following ensures than loading clickhouse data to a new cache result in the same
    // loaded data than our current cache
    let mut cache = CandleCache::default();
    let pairs =
        PairPrice::all_pairs(context.indexer_clickhouse_context.clickhouse_client()).await?;

    cache
        .preload_pairs(
            &pairs,
            context.indexer_clickhouse_context.clickhouse_client(),
        )
        .await?;

    let old_cache = context.indexer_clickhouse_context.candle_cache.read().await;

    // println!("Cache : {:#?}", old_cache.pair_prices);
    // println!("Cache from clickhouse: {:#?}", cache.pair_prices);
    assert_eq!(
        cache.pair_prices,
        // Filtering empty ones, since cache has them but not clickhouse
        old_cache
            .pair_prices
            .clone()
            .into_iter()
            .filter(|pp| !pp.1.is_empty())
            .collect::<HashMap<_, _>>()
    );

    // println!("Cache : {:#?}", old_cache.candles);
    // println!("Cache from clickhouse: {:#?}", cache.candles);
    assert_eq!(cache.candles, old_cache.candles);

    drop(old_cache);

    let mut suite_guard = suite.lock().await;
    suite_guard
        .app
        .indexer
        .shutdown()
        .expect("Can't shutdown indexer");

    Ok(())
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
                        price: NonZero::new_unchecked(price),
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

    Ok(())
}
