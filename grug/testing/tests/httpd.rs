use {
    assertor::*,
    grug_testing::{call_graphql, GraphQLCustomRequest, TestBuilder},
    grug_types::{Coins, Denom, Message, ResultExt},
    indexer_httpd::context::Context,
    std::str::FromStr,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_block() {
    let denom = Denom::from_str("ugrug").unwrap();

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .expect("Can't create indexer");

    let httpd_context: Context = indexer.context.clone().into();

    let (mut suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000).unwrap())
        .set_owner("owner")
        .build();

    let to = accounts["owner"].address;

    assert_that!(suite.app.indexer.indexing).is_true();

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            2000,
            Message::transfer(to, Coins::one(denom.clone(), 2_000).unwrap()).unwrap(),
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
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let response = call_graphql(httpd_context, request_body)
                    .await
                    .expect("Can't call graphql");

                assert_that!(response
                    .data
                    .as_object()
                    .unwrap()
                    .get("block")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("blockHeight")
                    .unwrap()
                    .as_u64()
                    .unwrap())
                .is_equal_to(1);
            })
            .await
            .unwrap();
        })
        .await;
}
