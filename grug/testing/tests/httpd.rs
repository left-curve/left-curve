use {
    assertor::*,
    grug_testing::{
        build_app_service, call_graphql, setup_tracing_subscriber, GraphQLCustomRequest,
        PaginatedResponse, TestAccounts, TestBuilder,
    },
    grug_types::{self, Coins, Denom, ResultExt},
    indexer_httpd::{
        context::Context,
        graphql::types::{block::Block, message::Message, transaction::Transaction},
    },
    std::str::FromStr,
};

async fn create_block() -> anyhow::Result<(Context, TestAccounts)> {
    setup_tracing_subscriber(tracing::Level::INFO);

    let denom = Denom::from_str("ugrug")?;

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .build()?;

    let httpd_context: Context = indexer.context.clone().into();

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000)?)
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;

    assert_that!(suite.app.indexer.indexing).is_true();

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            grug_types::Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap())?,
        )
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

    Ok((httpd_context, accounts))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_block() -> anyhow::Result<()> {
    let (httpd_context, _) = create_block().await?;

    let graphql_query = r#"
    query Block($height: Int!) {
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
        .await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_blocks() -> anyhow::Result<()> {
    let (httpd_context, _) = create_block().await?;

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
        .await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transactions() -> anyhow::Result<()> {
    let (httpd_context, accounts) = create_block().await?;

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
        .await??;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_messages() -> anyhow::Result<()> {
    let (httpd_context, accounts) = create_block().await?;

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
        .await??;

    Ok(())
}
