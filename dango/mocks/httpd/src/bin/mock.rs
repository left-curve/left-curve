use dango_mock_httpd::{BlockCreation, SetupValues};

#[tokio::main]
async fn main() {
    dango_mock_httpd::run(
        8080,
        BlockCreation::OnBroadcast,
        None,
        SetupValues::default(),
        true,
    )
    .await
    .unwrap();
}
