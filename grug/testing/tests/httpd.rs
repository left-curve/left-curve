use {
    assertor::*,
    grug_testing::{build_app_service, call_graphql, GraphQLCustomRequest, TestBuilder},
    grug_types::{Coins, Denom, Message, ResultExt},
    indexer_httpd::{context::Context, graphql::types::block::Block},
    std::str::FromStr,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_block() -> anyhow::Result<()> {
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
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap())?,
        )
        .should_succeed();

    // Force the runtime to wait for the async indexer task to finish
    suite.app.indexer.wait_for_finish();

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
