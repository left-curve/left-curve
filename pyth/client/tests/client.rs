mod common_function;
use {
    common_function::{test_latest_vaas, test_stream},
    pyth_client::PythClient,
    pyth_types::{BTC_USD_ID, ETH_USD_ID, PYTH_URL},
};

#[ignore = "Rely on network calls"]
#[test]
fn latest_vaas_network() {
    let pyth_client = PythClient::new(PYTH_URL).unwrap();
    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[ignore = "Rely on network calls"]
#[tokio::test]
async fn test_sse_stream() {
    let client = PythClient::new(PYTH_URL).unwrap();
    test_stream(client, vec![BTC_USD_ID, ETH_USD_ID]).await;
}
