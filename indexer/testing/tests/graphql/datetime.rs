use indexer_testing::{GraphQLCustomRequest, block::create_block, build_app_service, call_graphql};

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn graphql_returns_iso_8601() -> anyhow::Result<()> {
    // NOTE: It's necessary to capture the client in a variable named `_client`
    // here. It can't be named just an underscore (`_`) or dropped (`..`).
    // Otherwise, the indexer is dropped and the test fails.
    // You can see multiple instances of this throughout this file.
    let (httpd_context, _client, ..) = create_block().await?;

    let graphql_query = r#"
      query Block($height: Int) {
        block(height: $height) {
          id
          blockHeight
          appHash
          hash
          createdAt
          transactionsCount
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

                let response = call_graphql::<serde_json::Value>(app, request_body).await?;

                // Verify that `createdAt` is present and properly formatted as ISO 8601.
                let block = &response.data;
                let created_at = block
                    .get("createdAt")
                    .expect("`createdAt` field should exist");
                let created_at_str = created_at.as_str().expect("`createdAt` should be a string");

                // Verify that it ends with Z (UTC timezone indicator).
                assert!(
                    created_at_str.ends_with('Z'),
                    "`DateTime` should end with Z for UTC timezone: {}",
                    created_at_str
                );

                // Verify that it can be parsed as a valid RFC 3339 datetime.
                let _parsed = chrono::DateTime::parse_from_rfc3339(created_at_str)
                    .expect("`DateTime` should be valid RFC 3339 format");

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
