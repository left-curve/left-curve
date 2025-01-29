use {
    actix_web::{
        body::MessageBody,
        dev::{ServiceFactory, ServiceRequest, ServiceResponse},
        App,
    },
    assertor::*,
    dango_httpd::{
        graphql::{build_schema, types::transfer::Transfer},
        server::config_app,
    },
    dango_testing::setup_test_with_indexer,
    dango_types::{
        account::single,
        account_factory::{self, AccountParams},
        constants::USDC_DENOM,
    },
    grug::{
        build_actix_app_with_config, call_graphql, setup_tracing_subscriber, Coins,
        GraphQLCustomRequest, Message, NonEmpty, PaginatedResponse, ResultExt,
    },
    indexer_httpd::context::Context,
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
        Coins::one(USDC_DENOM.clone(), 100_000_000).unwrap(),
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
    query Transfers($block_height: Int!) {
      transfers(blockHeight: $block_height) {
        nodes {
          blockHeight
          fromAddress
          toAddress
          amount
          denom
        }
        edges { node { blockHeight fromAddress toAddress amount denom } cursor }
        pageInfo { hasPreviousPage hasNextPage startCursor endCursor }
      }
    }
    "#;

    let variables = serde_json::json!({
        "block_height": 1,
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
                    call_graphql::<PaginatedResponse<Transfer>>(app, request_body).await?;

                assert_that!(response.data.edges).has_length(2);

                assert_that!(response
                    .data
                    .edges
                    .iter()
                    .map(|t| t.node.block_height)
                    .collect::<Vec<_>>())
                .is_equal_to(vec![1, 1]);

                assert_that!(response
                    .data
                    .edges
                    .iter()
                    .map(|t| t.node.amount.as_str())
                    .collect::<Vec<_>>())
                .is_equal_to(vec!["100000000", "100000000"]);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await??;

    Ok(())
}

fn build_actix_app(
    app_ctx: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let graphql_schema = build_schema(app_ctx.clone());

    build_actix_app_with_config(app_ctx, graphql_schema, |app_ctx, graphql_schema| {
        config_app(app_ctx, graphql_schema)
    })
}
