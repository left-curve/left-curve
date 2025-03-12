mod common_function;
use {
    common_function::{test_latest_vaas, test_sse_streaming, test_stream},
    pyth_client::PythClient,
    pyth_types::{ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, ETH_USD_ID, PYTH_URL},
};

// Ignore since it makes network requests.
#[ignore]
#[test]
fn latest_vaas_network() {
    let pyth_client = PythClient::new(PYTH_URL).unwrap();

    // If this is used only here, could probably move that code here too
    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

// Ignore since it makes network requests.
#[ignore = "This test was testing stream, which is what test_sse_stream does now"]
#[test]
fn sse_subscription_network() {
    let pyth_client = PythClient::new(PYTH_URL).unwrap();

    test_sse_streaming(pyth_client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ]);
}

// Ignore since it makes network requests.
#[ignore]
#[tokio::test]
async fn test_sse_stream() {
    test_stream(vec![BTC_USD_ID, ETH_USD_ID]).await;
}
