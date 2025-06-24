use {
    assertor::*,
    grug_types::{Inner, Json, JsonSerExt, Query, QueryAppConfigRequest},
    indexer_testing::{GraphQLCustomRequest, block::create_block, build_app_service, call_graphql},
    serde_json::json,
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
