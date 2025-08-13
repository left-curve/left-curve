use {
    dango_genesis::GenesisOption,
    dango_mock_httpd::{BlockCreation, Error, TestOption},
    dango_testing::Preset,
    grug_testing::setup_tracing_subscriber,
    tracing::Level,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_tracing_subscriber(Level::INFO);
    dango_mock_httpd::run(
        8080,
        BlockCreation::OnBroadcast,
        None,
        TestOption {
            chain_id: "localdango-1".to_string(),
            ..Preset::preset_test()
        },
        GenesisOption::preset_test(),
        true,
        None,
    )
    .await
}
