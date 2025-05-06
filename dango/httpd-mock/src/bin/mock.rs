use {dango_httpd_mock::run_mock_httpd, dango_testing::SetupValues, grug_testing::BlockCreation};

#[tokio::main]
async fn main() {
    run_mock_httpd(
        8080,
        BlockCreation::OnBroadcast,
        None,
        SetupValues::default(),
        true,
    )
    .await
    .unwrap();
}
