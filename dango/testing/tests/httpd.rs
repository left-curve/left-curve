use {
    assertor::*,
    dango_httpd::graphql::types,
    dango_testing::{build_app_service, call_graphql, setup_test_with_indexer},
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
    },
    grug::{Coins, GraphQLCustomRequest, Message, NonEmpty, ResultExt},
    std::sync::Once,
};

static INIT: Once = Once::new();

fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .init();
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_transfer() -> anyhow::Result<()> {
    init_tracing();

    let ((mut suite, mut accounts, _, contracts), indexer_context) = setup_test_with_indexer();

    // Copied from benchmarks.rs
    let msgs = vec![Message::execute(
        contracts.account_factory,
        &account_factory::ExecuteMsg::RegisterAccount {
            params: AccountParams::Spot(single::Params::new(accounts.user1.username.clone())),
        },
        Coins::one("uusdc", 100_000_000).unwrap(),
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
                let app = build_app_service(indexer_context.into());

                let response =
                    call_graphql::<Vec<types::transfer::Transfer>>(app, request_body).await?;

                println!("{:#?}", response);

                // assert_that!(response.data.block_height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    Ok(())
}
