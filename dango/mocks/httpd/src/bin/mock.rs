use {
    dango_genesis::GenesisOption,
    dango_mock_httpd::{BlockCreation, Error, TestOption},
    dango_testing::Preset,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
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
