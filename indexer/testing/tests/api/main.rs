use {
    assert_json_diff::assert_json_include,
    assertor::*,
    grug_types::{Block, BlockOutcome},
    indexer_testing::{block::create_block, build_app_service, call_api, call_api_with_headers},
    serde_json::json,
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequesterIpResponse {
    remote_ip: Option<String>,
    peer_ip: Option<String>,
    x_forwarded_for: Option<String>,
    forwarded: Option<String>,
    cf_connecting_ip: Option<String>,
    true_client_ip: Option<String>,
    x_real_ip: Option<String>,
}

#[tokio::test(flavor = "multi_thread")]
async fn up_returns_200() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let up_response = call_api::<serde_json::Value>(app, "/up").await?;

                let expected = json!({
                    "block": { "height": 1 },
                    "is_running": true,
                    "indexed_block_height": 1,
                    "chain_id": "",
                });

                assert_json_include!(actual: up_response, expected: expected);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn api_returns_block() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context.clone());

                let block = call_api::<Block>(app, "/block/info/1").await?;
                assert_that!(block.info.height).is_equal_to(1);

                let app = build_app_service(httpd_context.clone());

                let block = call_api::<Block>(app, "/block/info").await?;
                assert_that!(block.info.height).is_equal_to(1);

                let app = build_app_service(httpd_context.clone());

                let block_outcome = call_api::<BlockOutcome>(app, "/block/result/1").await?;
                assert_that!(block_outcome.cron_outcomes).is_empty();
                assert_that!(block_outcome.tx_outcomes).has_length(1);

                let app = build_app_service(httpd_context.clone());

                let block_outcome = call_api::<BlockOutcome>(app, "/block/result").await?;
                assert_that!(block_outcome.cron_outcomes).is_empty();
                assert_that!(block_outcome.tx_outcomes).has_length(1);

                let app = build_app_service(httpd_context);

                let block_outcome = call_api::<BlockOutcome>(app, "/block/result/2").await;
                assert_that!(block_outcome).is_err();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread")]
async fn requester_ip_returns_forwarded_client_ip() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);
                let response = call_api_with_headers::<RequesterIpResponse>(
                    app,
                    "/requester-ip",
                    &[("X-Forwarded-For", "198.51.100.10, 127.0.0.1")],
                )
                .await?;

                assert_that!(response.remote_ip).is_equal_to(Some("198.51.100.10".to_string()));
                assert_that!(response.x_forwarded_for)
                    .is_equal_to(Some("198.51.100.10, 127.0.0.1".to_string()));
                assert_that!(response.peer_ip).is_some();
                assert_that!(response.forwarded).is_none();
                assert_that!(response.cf_connecting_ip).is_none();
                assert_that!(response.true_client_ip).is_none();
                assert_that!(response.x_real_ip).is_none();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
