use dango_mock_httpd::{BlockCreation, TestOption};

#[tokio::main]
async fn main() {
    dango_mock_httpd::run(
        8080,
        BlockCreation::OnBroadcast,
        None,
        TestOption::default(),
        true,
    )
    .await
    .unwrap();
}
