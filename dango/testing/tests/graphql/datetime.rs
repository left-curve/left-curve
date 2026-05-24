use {
    dango_testing::{
        TestOption, build_app_service, call_graphql_query,
        setup_test_naive_with_indexer_and_create_blocks,
    },
    graphql_client::GraphQLQuery,
    indexer_graphql_types::{Block, block},
};

#[tokio::test(flavor = "multi_thread")]
async fn graphql_returns_iso_8601() -> anyhow::Result<()> {
    let (_, _, httpd_context, _db_guard) =
        setup_test_naive_with_indexer_and_create_blocks(TestOption::default(), 1).await;

    let local_set = tokio::task::LocalSet::new();

    local_set
        .run_until(async {
            tokio::task::spawn_local(async {
                let app = build_app_service(httpd_context);
                let query_body = Block::build_query(block::Variables { height: Some(1) });

                let response =
                    call_graphql_query::<_, block::ResponseData, _, _, _>(app, query_body).await?;

                let block = response
                    .data
                    .expect("should have data")
                    .block
                    .expect("should have block");

                // Verify that `createdAt` is present and properly formatted as ISO 8601.
                let created_at = &block.created_at;

                // Verify that it ends with Z (UTC time zone indicator).
                assert!(
                    created_at.ends_with('Z'),
                    "`createdAt` should end with Z for UTC time zone: {created_at}"
                );

                // Verify that it can be parsed as a valid RFC 3339 datetime.
                assert!(
                    chrono::DateTime::parse_from_rfc3339(created_at).is_ok(),
                    "`createdAt` should be valid RFC 3339 format: {created_at}"
                );

                Ok::<(), anyhow::Error>(())
            })
            .await
        })
        .await?
}
