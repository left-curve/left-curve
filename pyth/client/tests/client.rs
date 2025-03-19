mod common_function;

use {
    common_function::{test_latest_vaas, test_stream},
    pyth_client::PythClient,
    pyth_types::{ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, ETH_USD_ID, PYTH_URL},
};

#[ignore = "rely on network calls"]
#[test]
fn latest_vaas_network() {
    let pyth_client = PythClient::new(PYTH_URL).unwrap();
    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[ignore = "rely on network calls"]
#[tokio::test]
async fn test_sse_stream() {
    let client = PythClient::new(PYTH_URL).unwrap();
    test_stream(client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ])
    .await;
}
