pub mod common_function;

use {
    crate::common_function::test_stream,
    grug::NonEmpty,
    pyth_lazer::PythClientLazer,
    pyth_types::constants::{
        ATOM_USD_ID_LAZER, BTC_USD_ID_LAZER, DOGE_USD_ID_LAZER, ETH_USD_ID_LAZER,
        LAZER_ACCESS_TOKEN_TEST, LAZER_ENDPOINTS_TEST,
    },
};

#[ignore = "rely on network calls"]
#[tokio::test]
async fn test_lazer_stream() {
    let client = PythClientLazer::new(
        NonEmpty::new_unchecked(LAZER_ENDPOINTS_TEST),
        LAZER_ACCESS_TOKEN_TEST,
    )
    .unwrap();
    test_stream(client, vec![BTC_USD_ID_LAZER, DOGE_USD_ID_LAZER], vec![
        ETH_USD_ID_LAZER,
        ATOM_USD_ID_LAZER,
    ])
    .await;
}
