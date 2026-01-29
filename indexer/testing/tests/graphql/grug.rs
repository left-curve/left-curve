use {
    assert_json_diff::assert_json_eq,
    assertor::*,
    graphql_client::GraphQLQuery,
    grug_types::{
        BroadcastClientExt, Coins, Denom, GasOption, Inner, Json, JsonSerExt, Message, Query,
        QueryAppConfigRequest, QueryBalanceRequest, ResultExt,
    },
    indexer_client::{QueryApp, SubscribeQueryApp, query_app, subscribe_query_app},
    indexer_testing::{
        GraphQLCustomRequest, block::create_block, build_app_service, call_graphql_query,
        call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    serde_json::json,
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_query_app() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let body_request = Query::AppConfig(QueryAppConfigRequest {}).to_json_value()?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async move {
                let variables = query_app::Variables {
                    request: body_request.into_inner(),
                    height: Some(1),
                };

                let app = build_app_service(httpd_context);
                let query_body = QueryApp::build_query(variables);

                let response =
                    call_graphql_query::<_, query_app::ResponseData, _, _, _>(app, query_body)
                        .await?;

                assert_that!(response.data).is_some();
                let data = response.data.unwrap();

                // Convert the JSON response for comparison
                let query_app_result: Json = serde_json::from_value(data.query_app)?;
                assert_that!(query_app_result.into_inner())
                    .is_equal_to(json!({"app_config": null}));

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn graphql_subscribe_to_query_app() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    // Use typed subscription from indexer-client
    let body_request = Query::Balance(QueryBalanceRequest {
        address: accounts["owner"].address,
        denom: Denom::from_str("ugrug")?,
    })
    .to_json_value()?;

    let request_body = GraphQLCustomRequest::from_query_body(
        SubscribeQueryApp::build_query(subscribe_query_app::Variables {
            request: body_request.into_inner(),
            block_interval: 1,
        }),
        "queryApp",
    );

    let (crate_block_tx, mut rx) = mpsc::channel::<u32>(1);

    // Can't call this from LocalSet so using channels instead.
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let to = accounts["owner"].address;
            let chain_id = client.chain_id().await;

            client
                .send_message(
                    &mut accounts["sender"],
                    Message::transfer(to, Coins::one(Denom::from_str("ugrug")?, 2_000)?)?,
                    GasOption::Predefined { gas_limit: 2000 },
                    &chain_id,
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

                // 1st response
                let response = parse_graphql_subscription_response::<
                    subscribe_query_app::SubscribeQueryAppQueryApp,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.block_height).is_equal_to(1);
                assert_json_eq!(
                    response.data.response,
                    json!({"balance": {"amount": "0", "denom": "ugrug"}})
                );

                crate_block_tx.send(2).await?;

                // 2nd response
                let response = parse_graphql_subscription_response::<
                    subscribe_query_app::SubscribeQueryAppQueryApp,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.block_height).is_equal_to(2);
                assert_json_eq!(
                    response.data.response,
                    json!({"balance": {"amount": "2000", "denom": "ugrug"}})
                );

                crate_block_tx.send(3).await?;

                // 3rd response
                let response = parse_graphql_subscription_response::<
                    subscribe_query_app::SubscribeQueryAppQueryApp,
                >(&mut framed, name)
                .await?;

                assert_that!(response.data.block_height).is_equal_to(3);
                assert_json_eq!(
                    response.data.response,
                    json!({"balance": {"amount": "4000", "denom": "ugrug"}})
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
