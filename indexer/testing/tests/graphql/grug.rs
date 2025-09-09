use {
    assert_json_diff::assert_json_eq,
    assertor::*,
    grug_types::{
        BroadcastClientExt, Coins, Denom, GasOption, Inner, Json, JsonSerExt, Message, Query,
        QueryAppConfigRequest, QueryBalanceRequest, ResultExt,
    },
    indexer_testing::{
        GraphQLCustomRequest, block::create_block, build_app_service, call_graphql,
        call_ws_graphql_stream, parse_graphql_subscription_response,
    },
    serde_json::json,
    std::str::FromStr,
    tokio::sync::mpsc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_query_app() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query QueryApp($request: String!, $height: Int!) {
        queryApp(request: $request, height: $height)
      }
    "#;

    let body_request = Query::AppConfig(QueryAppConfigRequest {}).to_json_value()?;

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

                let response = call_graphql::<Json, _, _, _>(app, request_body).await?;

                assert_that!(response.data.into_inner()).is_equal_to(json!({"app_config": null}));

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graphql_subscribe_to_query_app() -> anyhow::Result<()> {
    let (httpd_context, client, mut accounts) = create_block().await?;

    let graphql_query = r#"
      subscription QueryApp($request: String!) {
        queryApp(request: $request)
      }
    "#;

    let body_request = Query::Balance(QueryBalanceRequest {
        address: accounts["owner"].address,
        denom: Denom::from_str("ugrug")?,
    })
    .to_json_value()?;

    let variables = json!({
        "request": body_request,
    })
    .as_object()
    .unwrap()
    .clone();

    let request_body = GraphQLCustomRequest {
        name: "queryApp",
        query: graphql_query,
        variables,
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
                let response =
                    parse_graphql_subscription_response::<Json>(&mut framed, name).await?;

                assert_json_eq!(
                    response.data.into_inner(),
                    json!({"balance": {"amount": "0", "denom": "ugrug"}})
                );

                crate_block_tx.send(2).await?;

                // 2nd response
                let response =
                    parse_graphql_subscription_response::<Json>(&mut framed, name).await?;

                assert_json_eq!(
                    response.data.into_inner(),
                    json!({"balance": {"amount": "2000", "denom": "ugrug"}})
                );

                crate_block_tx.send(3).await?;

                // 3rd response
                let response =
                    parse_graphql_subscription_response::<Json>(&mut framed, name).await?;

                assert_json_eq!(
                    response.data.into_inner(),
                    json!({"balance": {"amount": "4000", "denom": "ugrug"}})
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
