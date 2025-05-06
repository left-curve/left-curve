use {dango_testing::SetupValues, grug_testing::BlockCreation, indexer_httpd_mock::run_mock_httpd};

#[tokio::main]
async fn main() {
    run_mock_httpd(
        8080,
        BlockCreation::OnBroadcast,
        None,
        SetupValues::default(),
    )
    .await
    .unwrap();
}
