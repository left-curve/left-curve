mod common_function;
use {
    common_function::{test_latest_vaas, test_sse_streaming},
    pyth_client::PythClient,
    pyth_types::{ATOM_USD_ID, BNB_USD_ID, BTC_USD_ID, ETH_USD_ID, PYTH_URL},
};

#[ignore]
#[test]
fn latest_vaas_network() {
    let pyth_client = PythClient::new(PYTH_URL);

    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[ignore]
#[test]
fn sse_subscription_network() {
    let pyth_client = PythClient::new(PYTH_URL);

    test_sse_streaming(pyth_client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ]);
}
