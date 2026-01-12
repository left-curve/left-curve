use {
    crate::build_actix_app,
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
        Addr, Addressable, Coin, Coins, Message, MultiplyFraction, NonEmpty, NonZero, NumberConst,
        ResultExt, Signer, StdResult, Timestamp, Udec128, Udec128_24, Uint128, btree_map,
    },
    grug_app::Indexer,
    indexer_client::{Trades, subscribe_trades, trades},
    indexer_testing::{
        GraphQLCustomRequest, call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    std::sync::Arc,
    tokio::sync::{Mutex, mpsc},
};

fn assert_trade(
    node: &trades::TradesTradesNodes,
    addr: Addr,
    direction: trades::Direction,
    filled_base: &str,
    filled_quote: &str,
    refund_base: &str,
    refund_quote: &str,
) {
    assert_that!(node.addr.as_str()).is_equal_to(addr.to_string().as_str());
    assert_that!(node.base_denom.as_str()).is_equal_to("dango");
    assert_that!(node.quote_denom.as_str()).is_equal_to("bridge/usdc");
    assert_that!(node.clearing_price.as_str()).is_equal_to("27.5");
    assert_that!(node.direction).is_equal_to(direction);
    assert_that!(node.time_in_force).is_equal_to(trades::TimeInForce::GTC);
    assert_that!(node.filled_base.as_str()).is_equal_to(filled_base);
    assert_that!(node.filled_quote.as_str()).is_equal_to(filled_quote);
    assert_that!(node.refund_base.as_str()).is_equal_to(refund_base);
    assert_that!(node.refund_quote.as_str()).is_equal_to(refund_quote);
}

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
                let nodes = &data.trades.nodes;

                assert_that!(nodes.len()).is_equal_to(4);

                assert_trade(
                    &nodes[0],
                    accounts.user6.address(),
                    trades::Direction::ask,
                    "5",
                    "137.5",
                    "0",
                    "136.95",
                );
                assert_trade(
                    &nodes[1],
                    accounts.user5.address(),
                    trades::Direction::ask,
                    "10",
                    "275",
                    "0",
                    "273.9",
                );
                assert_trade(
                    &nodes[2],
                    accounts.user4.address(),
                    trades::Direction::ask,
                    "10",
                    "275",
                    "0",
                    "273.9",
                );
                assert_trade(
                    &nodes[3],
                    accounts.user1.address(),
                    trades::Direction::bid,
                    "25",
                    "687.5",
                    "24.9",
                    "62.5",
                );

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
                let mut received_trades: Vec<trades::TradesTradesNodes> = Vec::new();
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
                        received_trades.push(node);
                    }

                    after = data.trades.page_info.end_cursor;

                    if after.is_none() {
                        break;
                    }
                }

                assert_that!(received_trades.len()).is_equal_to(4);

                assert_trade(
                    &received_trades[0],
                    accounts.user6.address(),
                    trades::Direction::ask,
                    "5",
                    "137.5",
                    "0",
                    "136.95",
                );
                assert_trade(
                    &received_trades[1],
                    accounts.user5.address(),
                    trades::Direction::ask,
                    "10",
                    "275",
                    "0",
                    "273.9",
                );
                assert_trade(
                    &received_trades[2],
                    accounts.user4.address(),
                    trades::Direction::ask,
                    "10",
                    "275",
                    "0",
                    "273.9",
                );
                assert_trade(
                    &received_trades[3],
                    accounts.user1.address(),
                    trades::Direction::bid,
                    "25",
                    "687.5",
                    "24.9",
                    "62.5",
                );

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
                let mut nodes: Vec<trades::TradesTradesNodes> = vec![];

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
                    nodes = data.trades.nodes;
                }

                assert_that!(nodes.len()).is_equal_to(1);

                let node = &nodes[0];
                assert_that!(node.addr.as_str())
                    .is_equal_to(accounts.user6.address().to_string().as_str());
                assert_that!(node.base_denom.as_str()).is_equal_to("dango");
                assert_that!(node.quote_denom.as_str()).is_equal_to("bridge/usdc");
                assert_that!(node.clearing_price.as_str()).is_equal_to("27.5");
                assert_that!(node.direction).is_equal_to(trades::Direction::ask);

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

    // Use typed subscription from indexer-client
    let graphql_query = r#"
      subscription Trades($baseDenom: String!, $quoteDenom: String!) {
          trades(baseDenom: $baseDenom, quoteDenom: $quoteDenom) {
            addr
            quoteDenom
            baseDenom
            direction
            timeInForce
            blockHeight
            createdAt
            filledBase
            filledQuote
            refundBase
            refundQuote
            feeBase
            feeQuote
            clearingPrice
          }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "trades",
        query: graphql_query,
        variables: [
            ("baseDenom".to_string(), serde_json::json!("dango")),
            ("quoteDenom".to_string(), serde_json::json!("bridge/usdc")),
        ]
        .into_iter()
        .collect(),
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

                let mut received_trades: Vec<subscribe_trades::SubscribeTradesTrades> = Vec::new();

                create_tx_clone.send(1).await.unwrap();

                // We should receive a total of 8 trades
                for _ in 1..=8 {
                    let response = parse_graphql_subscription_response::<
                        subscribe_trades::SubscribeTradesTrades,
                    >(&mut framed, name)
                    .await?;

                    received_trades.push(response.data);
                }

                // Verify we got 8 trades
                assert_that!(received_trades.len()).is_equal_to(8);

                // Expected: 4 trades at block 2 (1 bid, 3 ask), 4 trades at block 4 (1 bid, 3 ask)
                let expected_trades: Vec<(i64, subscribe_trades::Direction)> = vec![
                    (2, subscribe_trades::Direction::bid),
                    (2, subscribe_trades::Direction::ask),
                    (2, subscribe_trades::Direction::ask),
                    (2, subscribe_trades::Direction::ask),
                    (4, subscribe_trades::Direction::bid),
                    (4, subscribe_trades::Direction::ask),
                    (4, subscribe_trades::Direction::ask),
                    (4, subscribe_trades::Direction::ask),
                ];

                for (trade, (expected_block, expected_direction)) in
                    received_trades.iter().zip(expected_trades.iter())
                {
                    assert_eq!(trade.block_height, *expected_block);
                    assert_eq!(trade.direction, *expected_direction);
                    assert_eq!(trade.time_in_force, subscribe_trades::TimeInForce::GTC);
                }

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
