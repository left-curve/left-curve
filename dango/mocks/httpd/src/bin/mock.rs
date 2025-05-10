use dango_mock_httpd::{BlockCreation, Error, TestOption};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dango_mock_httpd::run(
        8080,
        BlockCreation::OnBroadcast,
        None,
        TestOption::default(),
        true,
        None,
    )
    .await
}
