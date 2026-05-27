use {
    assert_json_diff::assert_json_include,
    assertor::*,
    dango_genesis::GenesisOption,
    dango_testing::{
        BlockCreation, GraphQLCustomRequest, PaginationDirection, Preset, TestOption,
        build_app_service, call_graphql_query, call_ws_graphql_stream, mock_httpd_get_socket_addr,
        mock_httpd_run_with_callback, mock_httpd_wait_for_server_ready, paginate_transactions,
        parse_graphql_subscription_response, setup_test_naive_with_indexer_and_create_blocks,
        transactions_query,
    },
    dango_types::constants::usdc,
    graphql_client::GraphQLQuery,
    grug_types::{
        Addressable, Coins, Inner, JsonSerExt, MOCK_CHAIN_ID, Message, NonEmpty, ResultExt, Signer,
    },
    indexer_graphql_types::{
        Block, SubscribeTransactions, Transactions, block, subscribe_transactions, transactions,
    },
    indexer_sql::entity,
    sea_orm::EntityTrait,
    serde_json::json,
    std::time::Duration,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_last_block_transactions() -> anyhow::Result<()> {
    let (_, _, httpd_context, _db_guard) = setup_test_naive_with_indexer_and_create_blocks(
        TestOption::default().with_mocked_clickhouse(),
        1,
    )
    .await;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Block::build_query(block::Variables::default());

                let response =
                    call_graphql_query::<_, block::ResponseData, _, _, _>(app, query_body).await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();
                let block = data.block.unwrap();

                assert_that!(block.block_height).is_equal_to(1);
                assert_that!(block.transactions).has_length(1);
                assert_that!(block.transactions[0].block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_transactions() -> anyhow::Result<()> {
    let (_, accounts, httpd_context, _db_guard) = setup_test_naive_with_indexer_and_create_blocks(
        TestOption::default().with_mocked_clickhouse(),
        1,
    )
    .await;

    let sender_addr = accounts.user1.address().to_string();

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);
                let query_body = Transactions::build_query(transactions::Variables::default());

                let response =
                    call_graphql_query::<_, transactions::ResponseData, _, _, _>(app, query_body)
                        .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                assert_that!(data.transactions.nodes).has_length(1);
                assert_that!(data.transactions.nodes[0].sender.as_str())
                    .is_equal_to(sender_addr.as_str());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_paginate_transactions() -> anyhow::Result<()> {
    let (_, _, httpd_context, _db_guard) = setup_test_naive_with_indexer_and_create_blocks(
        TestOption::default().with_mocked_clickhouse(),
        10,
    )
    .await;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let page_size = 2;

                // 1. first with descending order
                let txs = paginate_transactions(
                    httpd_context.clone(),
                    page_size,
                    transactions_query::Variables {
                        sort_by: Some(transactions_query::TransactionSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = txs.iter().map(|t| t.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                // 2. first with ascending order
                let txs = paginate_transactions(
                    httpd_context.clone(),
                    page_size,
                    transactions_query::Variables {
                        sort_by: Some(transactions_query::TransactionSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Forward,
                )
                .await?;
                let block_heights: Vec<_> = txs.iter().map(|t| t.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 3. last with descending order
                let txs = paginate_transactions(
                    httpd_context.clone(),
                    page_size,
                    transactions_query::Variables {
                        sort_by: Some(transactions_query::TransactionSortBy::BLOCK_HEIGHT_DESC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = txs.iter().map(|t| t.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).collect::<Vec<_>>());

                // 4. last with ascending order
                let txs = paginate_transactions(
                    httpd_context.clone(),
                    page_size,
                    transactions_query::Variables {
                        sort_by: Some(transactions_query::TransactionSortBy::BLOCK_HEIGHT_ASC),
                        ..Default::default()
                    },
                    PaginationDirection::Backward,
                )
                .await?;
                let block_heights: Vec<_> = txs.iter().map(|t| t.block_height).collect();
                assert_that!(block_heights).is_equal_to((1i64..=10).rev().collect::<Vec<_>>());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_transactions() -> anyhow::Result<()> {
    let (mut suite, mut accounts, httpd_context, _db_guard) =
        setup_test_naive_with_indexer_and_create_blocks(
            TestOption::default().with_mocked_clickhouse(),
            1,
        )
        .await;

    // Use typed subscription from indexer-graphql-types
    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeTransactions::build_query(subscribe_transactions::Variables::default()),
        "transactions",
    );

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            suite
                .send_messages_with_gas(
                    &mut accounts.user1,
                    1_000_000,
                    NonEmpty::new_unchecked(vec![
                        Message::transfer(
                            accounts.user2.address(),
                            Coins::one(usdc::DENOM.clone(), 100).unwrap(),
                        )
                        .unwrap(),
                    ]),
                )
                .await
                .should_succeed();
        }
        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, mut framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transactions::SubscribeTransactionsTransactions>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transactions::SubscribeTransactionsTransactions>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    Vec<subscribe_transactions::SubscribeTransactionsTransactions>,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn transactions_stores_httpd_details() -> anyhow::Result<()> {
    let port = mock_httpd_get_socket_addr();

    let (sx, rx) = tokio::sync::oneshot::channel();
    let (sx2, rx2) = tokio::sync::oneshot::channel();

    // Run server in separate thread with its own runtime
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Starting mock HTTP server on port {port}");

            if let Err(error) = mock_httpd_run_with_callback(
                port,
                BlockCreation::OnBroadcast,
                None,
                TestOption::default(),
                GenesisOption::preset_test(),
                None,
                None,
                |accounts, _, _, _, indexer_context| {
                    sx.send(accounts).unwrap();
                    sx2.send(indexer_context).unwrap();
                },
            )
            .await
            {
                println!("Error running mock HTTP server: {error}");
            }
        });
    });

    let mut accounts = rx.await?;
    let indexer_context = rx2.await?;
    mock_httpd_wait_for_server_ready(port).await?;

    let tx = accounts.user1.sign_transaction(
        NonEmpty::new_unchecked(vec![Message::transfer(
            accounts.user2.address.into_inner(),
            Coins::one(usdc::DENOM.clone(), 100)?,
        )?]),
        MOCK_CHAIN_ID,
        1000000,
    )?;

    let response = reqwest::Client::new()
        .post(format!("http://localhost:{port}/graphql"))
        .header("X-Forwarded-For", "198.51.100.10, 127.0.0.1")
        .json(&json!({
            "query": "mutation BroadcastTxSync($tx: Tx!) { broadcastTxSync(tx: $tx) }",
            "variables": {
                "tx": tx.to_json_value()?.into_inner(),
            }
        }))
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("failed to submit GraphQL tx: {error}"))?;

    assert_that!(response.status().is_success()).is_true();

    // Transaction indexer is fully async and there is no way to know when it's finished
    for _ in 1..=30 {
        match entity::transactions::Entity::find()
            .one(&indexer_context.db)
            .await
            .expect("Can't fetch transaction")
        {
            Some(_) => {
                break;
            },
            None => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
        }
    }

    let http_request_details = entity::transactions::Entity::find()
        .one(&indexer_context.db)
        .await
        .expect("Can't fetch transaction")
        .expect("No transaction found")
        .http_request_details
        .expect("Can't find http_request_details");

    assert_json_include!(
        actual: http_request_details,
        expected: json!({
        "peer_ip": "127.0.0.1",
        "remote_ip": "198.51.100.10"
    }));

    Ok(())
}
