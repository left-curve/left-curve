use {
    crate::build_actix_app,
    assert_json_diff::assert_json_include,
    assertor::*,
    dango_genesis::Contracts,
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
    std::sync::Arc,
    tokio::sync::{Mutex, mpsc},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_all_trades() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Trades($addr: String) {
      trades(addr: $addr) {
          nodes {
            addr
            quoteDenom
            baseDenom
            direction
            orderType
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
          edges { node { addr quoteDenom baseDenom direction orderType filledBase filledQuote refundBase refundQuote feeBase feeQuote clearingPrice createdAt blockHeight }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "trades",
        query: graphql_query,
        variables: serde_json::json!({}).as_object().unwrap().clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_actix_app(dango_httpd_context.clone());

                let response: PaginatedResponse<serde_json::Value> =
                    call_paginated_graphql(app, request_body.clone()).await?;

                let received_trades = response
                    .edges
                    .into_iter()
                    .map(|e| e.node)
                    .collect::<Vec<_>>();

                let expected_candle = serde_json::json!([
                    {
                        "addr": accounts.user6.address(),
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "clearingPrice": "27.5",
                        "direction": "ask",
                        "orderType": "limit",
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
                        "orderType": "limit",
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
                        "orderType": "limit",
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
                        "orderType": "limit",
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_all_trades_with_pagination() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Trades($addr: String, $first: Int, $after: String) {
      trades(addr: $addr, first: $first, after: $after) {
          nodes {
            addr
            quoteDenom
            baseDenom
            direction
            orderType
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
          edges { node { addr quoteDenom baseDenom direction orderType filledBase filledQuote refundBase refundQuote feeBase feeQuote clearingPrice createdAt blockHeight }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut received_trades = Vec::new();
                let mut after = None;

                loop {
                    let app = build_actix_app(dango_httpd_context.clone());

                    let mut variables = serde_json::json!({"first": 1});
                    if let Some(cursor) = after {
                        variables["after"] = serde_json::json!(cursor);
                    }

                    let request_body = GraphQLCustomRequest {
                        name: "trades",
                        query: graphql_query,
                        variables: variables.as_object().unwrap().clone(),
                    };

                    let response: PaginatedResponse<serde_json::Value> =
                        call_paginated_graphql(app, request_body.clone()).await?;

                    received_trades.append(
                        &mut response
                            .edges
                            .into_iter()
                            .map(|e| e.node)
                            .collect::<Vec<_>>(),
                    );

                    after = response.page_info.end_cursor;

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
                        "orderType": "limit",
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
                        "orderType": "limit",
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
                        "orderType": "limit",
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
                        "orderType": "limit",
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_trades_with_address() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      query Trades($addr: String) {
      trades(addr: $addr) {
          nodes {
            addr
            quoteDenom
            baseDenom
            direction
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
          edges { node { addr quoteDenom baseDenom direction filledBase filledQuote refundBase refundQuote feeBase feeQuote clearingPrice createdAt blockHeight }  cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "trades",
        query: graphql_query,
        variables: serde_json::json!({"addr": accounts.user6.address()})
            .as_object()
            .unwrap()
            .clone(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let mut received_trades: Vec<serde_json::Value> = vec![];

                for _ in 0..10 {
                    let app = build_actix_app(dango_httpd_context.clone());

                    let response: PaginatedResponse<serde_json::Value> =
                        call_paginated_graphql(app, request_body.clone()).await?;

                    received_trades = response
                        .edges
                        .into_iter()
                        .map(|e| e.node)
                        .collect::<Vec<_>>();
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_trades() -> anyhow::Result<()> {
    let (mut suite, mut accounts, _, contracts, _, _, dango_httpd_context, _) =
        setup_test_with_indexer(TestOption::default()).await;

    create_pair_prices(&mut suite, &mut accounts, &contracts).await?;

    suite.app.indexer.wait_for_finish()?;

    let graphql_query = r#"
      subscription Trades($base_denom: String!, $quote_denom: String!) {
          trades(baseDenom: $base_denom, quoteDenom: $quote_denom) {
            addr
            quoteDenom
            baseDenom
            direction
            orderType
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

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish()?;
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

                // We should receive a total of 4 trades
                for _ in 1..=4 {
                    let response =
                        parse_graphql_subscription_response::<serde_json::Value>(&mut framed, name)
                            .await?;

                    received_trades.push(response.data);
                }

                let expected_json = serde_json::json!([
                    {
                        "baseDenom": "dango",
                        "quoteDenom": "bridge/usdc",
                        "direction": "bid",
                        "orderType": "limit",
                        "blockHeight": 4,
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

            let (funds, amount) = match direction {
                Direction::Bid => {
                    let quote_amount = amount.checked_mul_dec_ceil(price).unwrap();
                    (
                        Coins::one(usdc::DENOM.clone(), quote_amount).unwrap(),
                        quote_amount,
                    )
                },
                Direction::Ask => (Coins::one(dango::DENOM.clone(), amount).unwrap(), amount),
            };

            let msg = Message::execute(
                contracts.dex,
                &dex::ExecuteMsg::BatchUpdateOrders {
                    creates_market: vec![],
                    creates_limit: vec![match direction {
                        Direction::Bid => CreateLimitOrderRequest::Bid {
                            base_denom: dango::DENOM.clone(),
                            quote_denom: usdc::DENOM.clone(),
                            amount_quote: NonZero::new_unchecked(amount),
                            price: NonZero::new_unchecked(price),
                        },
                        Direction::Ask => CreateLimitOrderRequest::Ask {
                            base_denom: dango::DENOM.clone(),
                            quote_denom: usdc::DENOM.clone(),
                            amount_base: NonZero::new_unchecked(amount),
                            price: NonZero::new_unchecked(price),
                        },
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
