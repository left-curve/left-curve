use {
    assertor::*,
    grug_types::{Block, BlockOutcome},
    indexer_testing::{block::create_block, build_app_service, call_api},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn up_returns_200() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let up_response = call_api::<serde_json::Value>(app, "/up").await?;

                assert_that!(
                    up_response
                        .get("block_height")
                        .and_then(|bh| bh.as_u64())
                        .unwrap_or_default()
                )
                .is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_returns_block() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context.clone());

                let block = call_api::<Block>(app, "/api/block/info/1").await?;
                assert_that!(block.info.height).is_equal_to(1);

                let app = build_app_service(httpd_context.clone());

                let block = call_api::<Block>(app, "/api/block/info").await?;
                assert_that!(block.info.height).is_equal_to(1);

                let app = build_app_service(httpd_context.clone());

                let block_outcome = call_api::<BlockOutcome>(app, "/api/block/result/1").await?;
                assert_that!(block_outcome.cron_outcomes).is_empty();
                assert_that!(block_outcome.tx_outcomes).has_length(1);

                let app = build_app_service(httpd_context.clone());

                let block_outcome = call_api::<BlockOutcome>(app, "/api/block/result").await?;
                assert_that!(block_outcome.cron_outcomes).is_empty();
                assert_that!(block_outcome.tx_outcomes).has_length(1);

                let app = build_app_service(httpd_context);

                let block_outcome = call_api::<BlockOutcome>(app, "/api/block/result/2").await;
                assert_that!(block_outcome).is_err();

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
