use {
    assert_json_diff::assert_json_include,
    assertor::*,
    grug_app::NaiveProposalPreparer,
    grug_db_memory::MemDb,
    grug_testing::{MockClient, TestAccounts, TestBuilder, setup_tracing_subscriber},
    grug_types::{BroadcastClientExt, Coins, Denom, JsonSerExt, ResultExt},
    grug_vm_rust::RustVm,
    indexer_httpd::{
        context::Context,
        graphql::types::{block::Block, event::Event, message::Message, transaction::Transaction},
        traits::QueryApp,
    },
    indexer_sql::{hooks::NullHooks, non_blocking_indexer::NonBlockingIndexer},
    indexer_testing::{
        GraphQLCustomRequest, PaginatedResponse, build_app_service, call_api, call_graphql,
        call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    serde_json::json,
    std::{str::FromStr, sync::Arc},
    tokio::sync::{Mutex, mpsc},
};

async fn create_block() -> anyhow::Result<(
    Context,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, NonBlockingIndexer<NullHooks>>>,
    TestAccounts,
)> {
    setup_tracing_subscriber(tracing::Level::INFO);

    let denom = Denom::from_str("ugrug")?;

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_keep_blocks(true)
        .build()?;

    let context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let (suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000)?)
        .set_owner("owner")
        .build();

    let chain_id = suite.chain_id().await?;

    let suite = Arc::new(Mutex::new(suite));

    let mock_client =
        MockClient::new_shared(suite.clone(), grug_testing::BlockCreation::OnBroadcast);

    let sender = accounts["sender"].address;

    mock_client
        .send_message(
            &mut accounts["sender"],
            grug_types::Message::transfer(sender, Coins::one(denom.clone(), 2_000)?)?,
            grug_types::GasOption::Predefined { gas_limit: 2000 },
            &chain_id,
        )
        .await?;

    suite.lock().await.app.indexer.wait_for_finish();

    assert_that!(suite.lock().await.app.indexer.indexing).is_true();

    let client = Arc::new(mock_client);

    let httpd_context = Context::new(context, suite, client.clone(), indexer_path);

    Ok((httpd_context, client, accounts))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn graphql_returns_block() -> anyhow::Result<()> {
    // NOTE: It's necessary to capture the client in a variable named `_client`
    // here. It can't be named just an underscore (`_`) or dropped (`..`).
    // Otherwise, the indexer is dropped and the test fails.
    // You can see multiple instances of this throughout this file.
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block($height: Int) {
        block(height: $height) {
          blockHeight
          appHash
          hash
          createdAt
        }
      }
    "#;

    let variables = serde_json::json!({
        "height": 1,
    })
    .as_object()
    .unwrap()
    .to_owned();

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<Block>(app, request_body).await?;

                assert_that!(response.data.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_returns_block() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context.clone());

                let block: grug_types::Block = call_api(app, "/api/block/info/1").await?;

                assert_that!(block.info.height).is_equal_to(1);

                let app = build_app_service(httpd_context.clone());

                let block: grug_types::Block = call_api(app, "/api/block/info").await?;

                assert_that!(block.info.height).is_equal_to(1);

                let app = build_app_service(httpd_context.clone());

                let block_outcome: grug_types::BlockOutcome =
                    call_api(app, "/api/block/result/1").await?;

                assert_that!(block_outcome.cron_outcomes).is_empty();
                assert_that!(block_outcome.tx_outcomes).has_length(1);

                let app = build_app_service(httpd_context.clone());

                let block_outcome: grug_types::BlockOutcome =
                    call_api(app, "/api/block/result").await?;

                assert_that!(block_outcome.cron_outcomes).is_empty();
                assert_that!(block_outcome.tx_outcomes).has_length(1);

                let app = build_app_service(httpd_context);

                let block_outcome: Result<grug_types::BlockOutcome, _> =
                    call_api(app, "/api/block/result/2").await;

                assert_that!(block_outcome).is_err();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_last_block() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block {
        block {
          blockHeight
          appHash
          hash
          createdAt
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<Block>(app, request_body).await?;

                assert_that!(response.data.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_last_block_transactions() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block {
        block {
          blockHeight
          transactions {
            blockHeight
          }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<serde_json::Value>(app, request_body).await?;

                let expected = json!({
                    "blockHeight": 1,
                    "transactions": [
                        {
                            "blockHeight": 1,
                        }
                    ]
                });

                assert_json_include!(actual: response.data, expected: expected);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_blocks() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Blocks {
        blocks {
          nodes {
            blockHeight
            appHash
            hash
            createdAt
          }
          edges { node { blockHeight appHash hash createdAt } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "blocks",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<PaginatedResponse<Block>>(app, request_body).await?;

                assert_that!(response.data.edges).has_length(1);
                assert_that!(response.data.edges[0].node.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transactions() -> anyhow::Result<()> {
    let (httpd_context, _client, accounts) = create_block().await?;

    let graphql_query = r#"
      query Transactions {
        transactions {
          nodes {
            blockHeight
            sender
            hash
            hasSucceeded
          }
          edges { node { blockHeight sender hash hasSucceeded } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transactions",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<Transaction>>(app, request_body).await?;

                assert_that!(response.data.edges).has_length(1);

                assert_that!(response.data.edges[0].node.sender)
                    .is_equal_to(accounts["sender"].address.to_string());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_nested_events() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Transactions {
        transactions {
          nodes {
            blockHeight
            sender
            hash
            hasSucceeded
            nestedEvents
          }
          edges { node { blockHeight sender hash hasSucceeded nestedEvents } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transactions",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<serde_json::Value>>(app, request_body).await?;

                let nested_events: &str = response.data.edges[0]
                    .node
                    .get("nestedEvents")
                    .and_then(|s| s.as_str())
                    .expect("Can't get nestedEvents");

                assert_that!(nested_events.len()).is_at_least(10);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_messages() -> anyhow::Result<()> {
    let (httpd_context, _client, accounts) = create_block().await?;

    let graphql_query = r#"
      query Messages {
        messages {
          nodes {
            blockHeight
            methodName
            contractAddr
            senderAddr
          }
          edges { node { blockHeight methodName contractAddr senderAddr } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "messages",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response =
                    call_graphql::<PaginatedResponse<Message>>(app, request_body).await?;

                assert_that!(response.data.edges).has_length(1);

                assert_that!(response.data.edges[0].node.sender_addr)
                    .is_equal_to(accounts["sender"].address.to_string());

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_events() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Events {
        events {
          nodes {
            blockHeight
            createdAt
            eventIdx
            type
            method
            eventStatus
            commitmentStatus
            data
          }
          edges { node { blockHeight createdAt eventIdx type method eventStatus commitmentStatus data } cursor }
          pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "events",
        query: graphql_query,
        variables: Default::default(),
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<PaginatedResponse<Event>>(app, request_body).await?;

                assert_that!(response.data.edges).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_block() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Block {
        block {
          blockHeight
          createdAt
          hash
          appHash
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "block",
        query: graphql_query,
        variables: Default::default(),
    };

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;
            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    grug_types::Message::transfer(
                        to,
                        Coins::one(Denom::from_str("ugrug")?, 2_000)?,
                    )?,
                    grug_types::GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
                )
                .await
                .should_succeed();

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }

        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) =
                    parse_graphql_subscription_response::<Block>(framed, name).await?;

                assert_that!(response.data.block_height).is_equal_to(1);

                crate_block_tx.send(2).await?;

                // 2st response
                let (framed, response) =
                    parse_graphql_subscription_response::<Block>(framed, name).await?;

                assert_that!(response.data.block_height).is_equal_to(2);

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) =
                    parse_graphql_subscription_response::<Block>(framed, name).await?;

                assert_that!(response.data.block_height).is_equal_to(3);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_transactions() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Transactions {
        transactions {
          blockHeight
          createdAt
          transactionType
          transactionIdx
          sender
          hash
          hasSucceeded
          errorMessage
          gasWanted
          gasUsed
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "transactions",
        query: graphql_query,
        variables: Default::default(),
    };

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;
            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    grug_types::Message::transfer(
                        to,
                        Coins::one(Denom::from_str("ugrug")?, 2_000)?,
                    )?,
                    grug_types::GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
                )
                .await
                .should_succeed();

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }

        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<Transaction>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(2).await?;

                // 2st response
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<Transaction>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) =
                    parse_graphql_subscription_response::<Vec<Transaction>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_messages() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Messages {
        messages {
          blockHeight
          createdAt
          orderIdx
          methodName
          contractAddr
          senderAddr
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "messages",
        query: graphql_query,
        variables: Default::default(),
    };

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    let owner_addr = accounts["sender"].address.to_string();

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;

            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    grug_types::Message::transfer(
                        to,
                        Coins::one(Denom::from_str("ugrug")?, 2_000)?,
                    )?,
                    grug_types::GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
                )
                .await
                .should_succeed();

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }

        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<Message>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(2).await?;

                // 2st response
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<Message>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) =
                    parse_graphql_subscription_response::<Vec<Message>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data.first().unwrap().method_name.as_str())
                    .is_equal_to("transfer");
                assert_that!(response.data.first().unwrap().sender_addr.as_str())
                    .is_equal_to(owner_addr.as_str());
                assert_that!(response.data).has_length(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_events() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription Events {
        events {
          blockHeight
          createdAt
          eventIdx
          type
          method
          eventStatus
          commitmentStatus
          data
        }
      }
    "#;

    let request_body = GraphQLCustomRequest {
        name: "events",
        query: graphql_query,
        variables: Default::default(),
    };

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;

            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    grug_types::Message::transfer(
                        to,
                        Coins::one(Denom::from_str("ugrug")?, 2_000)?,
                    )?,
                    grug_types::GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
                )
                .await
                .should_succeed();

            // Enabling this here will cause the test to hang
            // suite.app.indexer.wait_for_finish();
        }

        Ok::<(), anyhow::Error>(())
    });

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let name = request_body.name;
                let (_srv, _ws, framed) =
                    call_ws_graphql_stream(httpd_context, build_app_service, request_body).await?;

                // 1st response is always the existing last block
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<Event>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(1);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(2).await?;

                // 2st response
                let (framed, response) =
                    parse_graphql_subscription_response::<Vec<Message>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(2);
                assert_that!(response.data).is_not_empty();

                crate_block_tx.send(3).await?;

                // 3rd response
                let (_, response) =
                    parse_graphql_subscription_response::<Vec<Message>>(framed, name).await?;

                assert_that!(response.data.first().unwrap().block_height).is_equal_to(3);
                assert_that!(response.data).is_not_empty();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_query_app() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query QueryApp($request: String!, $height: Int!) {
        queryApp(request: $request, height: $height)
      }
    "#;

    let body_request =
        grug_types::Query::AppConfig(grug_types::QueryAppConfigRequest {}).to_json_string()?;

    let variables = json!({
        "request": body_request,
        "height": 1,
    })
    .as_object()
    .unwrap()
    .clone();

    let request_body = GraphQLCustomRequest {
        name: "queryApp",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let response = call_graphql::<String>(app, request_body).await?;

                assert_that!(response.data.as_str()).is_equal_to("{\"app_config\":null}");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
