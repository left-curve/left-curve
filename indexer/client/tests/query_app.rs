use {
    assertor::*,
    grug_types::Block,
    indexer_testing::{block::create_block, build_app_service, call_api},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graphql_returns_config() -> anyhow::Result<()> {
    let (httpd_context, _client, ..) = create_block().await?;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);

                let block = call_api::<Block>(app, "/api/block/info/1").await?;
                assert_that!(block.info.height).is_equal_to(1);

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
