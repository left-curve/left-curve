use {
    assertor::*,
    dango_httpd::{graphql::types, server::build_actix_app},
    dango_testing::setup_test_with_indexer,
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
    },
    grug::{
        call_graphql, setup_tracing_subscriber, Coins, GraphQLCustomRequest, Message, NonEmpty,
        ResultExt,
    },
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transfer() -> anyhow::Result<()> {
    setup_tracing_subscriber(tracing::Level::INFO);

    let ((mut suite, mut accounts, _, contracts), indexer_context) = setup_test_with_indexer();

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
        },
        Coins::one("hyp/eth/usdc", 100_000_000).unwrap(),
    )?];

    suite
        .send_messages_with_gas(
            &mut accounts.user1,
            50_000_000,
            NonEmpty::new_unchecked(msgs),
        )
        .should_succeed();

    suite.app.indexer.wait_for_finish();

    let graphql_query = r#"
    query Transfers($height: Int!) {
      transfers(height: $height) {
        blockHeight
        fromAddress
        toAddress
        amount
        denom
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
        name: "transfers",
        query: graphql_query,
        variables,
    };

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_actix_app(indexer_context.into());

                let response =
                    call_graphql::<Vec<types::transfer::Transfer>>(app, request_body).await?;

                assert_that!(response
                    .data
                    .iter()
                    .map(|t| t.block_height)
                    .collect::<Vec<_>>())
                .is_equal_to(vec![1, 1]);

                assert_that!(response
                    .data
                    .iter()
                    .map(|t| t.amount.as_str())
                    .collect::<Vec<_>>())
                .is_equal_to(vec!["100000000", "100000000"]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    Ok(())
}
