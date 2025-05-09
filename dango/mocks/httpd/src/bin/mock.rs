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
        TestOption::default(),
        GenesisOption::preset_test(),
        true,
    )
    .await
}
