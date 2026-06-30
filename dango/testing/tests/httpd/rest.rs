use {
    assert_json_diff::assert_json_include,
    assertor::*,
    dango_genesis::GenesisOption,
    dango_indexer_sql::entity,
    dango_primitives::{
        Coins, Inner, JsonSerExt, MOCK_CHAIN_ID, Message, NonEmpty, Query, QueryAppConfigRequest,
        QueryResponse, Signer, TxOutcome,
    },
    dango_testing::{
        BlockCreation, Preset, TestOption, call_rest_post_with_context, mock_httpd_get_socket_addr,
        mock_httpd_run_with_callback, mock_httpd_wait_for_server_ready,
        setup_test_naive_with_indexer_and_create_blocks,
    },
    dango_types::constants::usdc,
    sea_orm::EntityTrait,
    serde_json::json,
    std::time::Duration,
};

#[tokio::test(flavor = "multi_thread")]
async fn rest_query_returns_app_config() -> anyhow::Result<()> {
    let (_, _, httpd_context, _db_guard) = setup_test_naive_with_indexer_and_create_blocks(
        TestOption::default().with_mocked_clickhouse(),
        1,
    )
    .await;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let response: QueryResponse = call_rest_post_with_context(
                    httpd_context,
                    "/query",
                    &Query::AppConfig(QueryAppConfigRequest {}),
                    &[],
                )
                .await?;

                match response {
                    QueryResponse::AppConfig(config) => {
                        assert_that!(config.into_inner()).is_not_equal_to(serde_json::Value::Null);
                    },
                    other => panic!("unexpected query response: {other:?}"),
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn rest_simulate_returns_outcome() -> anyhow::Result<()> {
    let (_, accounts, httpd_context, _db_guard) = setup_test_naive_with_indexer_and_create_blocks(
        TestOption::default().with_mocked_clickhouse(),
        1,
    )
    .await;

    let unsigned = accounts.user1.unsigned_transaction(
        NonEmpty::new_unchecked(vec![Message::transfer(
            accounts.user2.address.into_inner(),
            Coins::one(usdc::DENOM.clone(), 100)?,
        )?]),
        MOCK_CHAIN_ID,
    )?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let outcome: TxOutcome =
                    call_rest_post_with_context(httpd_context, "/simulate", &unsigned, &[]).await?;

                // A dry-run always consumes gas, even though the signature is
                // not verified.
                assert_that!(outcome.gas_used > 0).is_true();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

/// Broadcasting over REST must capture the requester's IP the same way the
/// GraphQL `broadcastTxSync` mutation does (both go through the shared
/// `crate::broadcast::broadcast_tx` helper).
#[tokio::test(flavor = "multi_thread")]
async fn rest_broadcast_stores_httpd_details() -> anyhow::Result<()> {
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
        .post(format!("http://localhost:{port}/broadcast"))
        .header("X-Forwarded-For", "198.51.100.10, 127.0.0.1")
        .json(&tx.to_json_value()?.into_inner())
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("failed to submit REST tx: {error}"))?;

    assert_that!(response.status().is_success()).is_true();

    // Transaction indexer is fully async and there is no way to know when it's
    // finished
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
        })
    );

    Ok(())
}
