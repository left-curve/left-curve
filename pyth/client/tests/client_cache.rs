mod common_function;

use {
    common_function::{test_latest_vaas, test_sse_streaming, test_stream},
    grug::__private::borsh::de,
    pyth_client::{client_cache::PythClientCache, PythClient},
    pyth_types::{ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, ETH_USD_ID, PYTH_URL},
    std::vec,
};

#[test]
fn latest_vaas_cache() {
    let pyth_client = PythClientCache::new(PYTH_URL).unwrap();

    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[tokio::test]
async fn test_sse_stream_cache() {
    let client = PythClientCache::new(PYTH_URL).unwrap();
    test_stream(client, vec![BTC_USD_ID, ETH_USD_ID]).await;
}

#[deprecated]
#[test]
fn sse_subscription_cache() {
    // NOTE: should use PythMiddlewareCache and refactor test_sse_streaming
    let pyth_client = PythClient::new(PYTH_URL).unwrap();

    test_sse_streaming(pyth_client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ]);
}
