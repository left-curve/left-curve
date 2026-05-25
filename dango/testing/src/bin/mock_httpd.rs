use {
    dango_genesis::GenesisOption,
    dango_testing::{BlockCreation, HttpdError, Preset, TestOption},
};

#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), HttpdError> {
    dango_testing::mock_httpd_run(
        8080,
        BlockCreation::OnBroadcast,
        None,
        TestOption {
            chain_id: "localdango-1".to_string(),
            ..Preset::preset_test()
        },
        GenesisOption::preset_test(),
        None,
    )
    .await
}
