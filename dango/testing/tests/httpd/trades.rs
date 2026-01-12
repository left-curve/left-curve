use {
    crate::build_actix_app,
    assert_json_diff::assert_json_include,
    assertor::*,
    dango_genesis::Contracts,
    dango_testing::{TestAccounts, TestOption, TestSuiteWithIndexer, setup_test_with_indexer},
    dango_types::{
        constants::{dango, usdc},
        dex::{self, CreateOrderRequest, Direction},
        oracle::{self, PriceSource},
    },
    graphql_client::{GraphQLQuery, Response},
    grug::{
        Addressable, Coin, Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst,
        ResultExt, Signer, StdResult, Timestamp, Udec128, Udec128_24, Uint128, btree_map,
    },
    grug_app::Indexer,
    indexer_client::{Trades, trades},
    indexer_testing::{
        GraphQLCustomRequest, call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    std::sync::Arc,
    tokio::sync::{Mutex, mpsc},
};

#[tokio::test(flavor = "multi_thread")]
async fn query_all_trades() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let request_body = Trades::build_query(trades::Variables::default());

                let app = build_actix_app(dango_httpd_context);
                let app = actix_web::test::init_service(app).await;

                let request = actix_web::test::TestRequest::post()
                    .uri("/graphql")
                    .set_json(&request_body)
                    .to_request();

                let response = actix_web::test::call_and_read_body(&app, request).await;
                let response: Response<trades::ResponseData> = serde_json::from_slice(&response)?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                let received_trades: Vec<_> = data
                    .trades
                    .nodes
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "addr": t.addr,
                            "baseDenom": t.base_denom,
                            "quoteDenom": t.quote_denom,
                            "clearingPrice": t.clearing_price,
                            "direction": format!("{:?}", t.direction).to_lowercase(),
                            "timeInForce": format!("{:?}", t.time_in_force),
                            "filledBase": t.filled_base,
                            "filledQuote": t.filled_quote,
                            "refundBase": t.refund_base,
                            "refundQuote": t.refund_quote,
                        })
                    })
                    .collect();

                let expected_candle = serde_json::json!([
                    {
                        "addr": accounts.user6.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "timeInForce": "GTC",
                        "filledBase": "5",
                        "filledQuote": "137.5",
                        "refundBase": "0",
                        "refundQuote": "136.95",
                    },
                    {
                        "addr": accounts.user5.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "timeInForce": "GTC",
                        "filledBase": "10",
                        "filledQuote": "275",
                        "refundBase": "0",
                        "refundQuote": "273.9",
                    },
                    {
                        "addr": accounts.user4.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "timeInForce": "GTC",
                        "filledBase": "10",
                        "filledQuote": "275",
                        "refundBase": "0",
                        "refundQuote": "273.9",
                    },
                    {
                        "addr": accounts.user1.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "bid",
                        "timeInForce": "GTC",
                        "filledBase": "25",
                        "filledQuote": "687.5",
                        "refundBase": "24.9",
                        "refundQuote": "62.5",
                    },
                ]);

                assert_json_include!(actual: received_trades, expected: expected_candle);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_all_trades_with_pagination() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut received_trades = Vec::new();
                let mut after = None;

                loop {
                    let variables = trades::Variables {
                        after: after.clone(),
                        first: Some(1),
                        ..Default::default()
                    };

                    let request_body = Trades::build_query(variables);
                    let app = build_actix_app(dango_httpd_context.clone());
                    let app = actix_web::test::init_service(app).await;

                    let request = actix_web::test::TestRequest::post()
                        .uri("/graphql")
                        .set_json(&request_body)
                        .to_request();

                    let response = actix_web::test::call_and_read_body(&app, request).await;
                    let response: Response<trades::ResponseData> =
                        serde_json::from_slice(&response)?;

                    let data = response.data.unwrap();

                    for node in data.trades.nodes {
                        received_trades.push(serde_json::json!({
                            "addr": node.addr,
                            "baseDenom": node.base_denom,
                            "quoteDenom": node.quote_denom,
                            "clearingPrice": node.clearing_price,
                            "direction": format!("{:?}", node.direction).to_lowercase(),
                            "timeInForce": format!("{:?}", node.time_in_force),
                            "filledBase": node.filled_base,
                            "filledQuote": node.filled_quote,
                            "refundBase": node.refund_base,
                            "refundQuote": node.refund_quote,
                        }));
                    }

                    after = data.trades.page_info.end_cursor;

                    if after.is_none() {
                        break;
                    }
                }

                let expected_candle = serde_json::json!([
                    {
                        "addr": accounts.user6.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "timeInForce": "GTC",
                        "filledBase": "5",
                        "filledQuote": "137.5",
                        "refundBase": "0",
                        "refundQuote": "136.95",
                    },
                    {
                        "addr": accounts.user5.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "timeInForce": "GTC",
                        "filledBase": "10",
                        "filledQuote": "275",
                        "refundBase": "0",
                        "refundQuote": "273.9",
                    },
                    {
                        "addr": accounts.user4.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "timeInForce": "GTC",
                        "filledBase": "10",
                        "filledQuote": "275",
                        "refundBase": "0",
                        "refundQuote": "273.9",
                    },
                    {
                        "addr": accounts.user1.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "bid",
                        "timeInForce": "GTC",
                        "filledBase": "25",
                        "filledQuote": "687.5",
                        "refundBase": "24.9",
                        "refundQuote": "62.5",
                    },
                ]);

                assert_json_include!(actual: received_trades, expected: expected_candle);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn query_trades_with_address() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut received_trades: Vec<serde_json::Value> = vec![];

                for _ in 0..10 {
                    let variables = trades::Variables {
                        addr: Some(accounts.user6.address().to_string()),
                        ..Default::default()
                    };

                    let request_body = Trades::build_query(variables);
                    let app = build_actix_app(dango_httpd_context.clone());
                    let app = actix_web::test::init_service(app).await;

                    let request = actix_web::test::TestRequest::post()
                        .uri("/graphql")
                        .set_json(&request_body)
                        .to_request();

                    let response = actix_web::test::call_and_read_body(&app, request).await;
                    let response: Response<trades::ResponseData> =
                        serde_json::from_slice(&response)?;

                    let data = response.data.unwrap();

                    received_trades = data
                        .trades
                        .nodes
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "addr": t.addr,
                                "baseDenom": t.base_denom,
                                "quoteDenom": t.quote_denom,
                                "clearingPrice": t.clearing_price,
                                "direction": format!("{:?}", t.direction).to_lowercase(),
                            })
                        })
                        .collect();
                }

                let expected_candle = serde_json::json!([
                    {
                        "addr": accounts.user6.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                    },
                ]);

                assert_json_include!(actual: received_trades, expected: expected_candle);
                assert_that!(received_trades).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_trades() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _, _db_guard) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish().await?;

    // Subscriptions still use raw GraphQL since indexer-client doesn't support them yet
    let graphql_query = r#"
      subscription Trades($base_denom: String!, $quote_denom: String!) {
          trades(baseDenom: $base_denom, quoteDenom: $quote_denom) {
            addr
            quoteDenom
            baseDenom
            direction
            timeInForce
            filledBase
            filledQuote
            refundBase
            refundQuote
            feeBase
            feeQuote
            clearingPrice
            createdAt
            blockHeight
          }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "trades",
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
    let suite = Arc::new(Mutex::new(suite));
    let suite_clone = suite.clone();

    // Can't call this from LocalSet so using channels instead.
    let (create_tx, mut rx) = mpsc::channel::<u32>(1);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let mut suite_guard = suite_clone.lock().await;

            create_pair_prices(&mut suite_guard, &mut accounts, &contracts).await?;
        }

        Ok::<(), anyhow::Error>(())
    });

    let create_tx_clone = create_tx.clone();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(dango_httpd_context, build_actix_app, request_body)
                        .await?;

                let mut received_trades = Vec::new();

                create_tx_clone.send(1).await.unwrap();

                // We should receive a total of 8 trades
                for _ in 1..=8 {
                    let response =
                        parse_graphql_subscription_response::<serde_json::Value>(&mut framed, name)
                            .await?;

                    received_trades.push(response.data);
                }

                let expected_json = serde_json::json!([
                    {
                        "blockHeight": 2,
                        "direction": "bid",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 2,
                        "direction": "ask",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 2,
                        "direction": "ask",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 2,
                        "direction": "ask",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 4,
                        "direction": "bid",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 4,
                        "direction": "ask",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 4,
                        "direction": "ask",
                        "timeInForce": "GTC",
                    },
                    {
                        "blockHeight": 4,
                        "direction": "ask",
                        "timeInForce": "GTC",
                    },
                ]);

                assert_json_include!(actual: received_trades, expected: expected_json);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

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
