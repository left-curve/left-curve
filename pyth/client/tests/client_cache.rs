mod common_function;

use {
    common_function::{test_latest_vaas, test_stream},
    pyth_client::client_cache::PythClientCache,
    pyth_types::{BTC_USD_ID, ETH_USD_ID, PYTH_URL},
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
