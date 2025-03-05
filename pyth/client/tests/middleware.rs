mod common_function;

use {
    common_function::{test_latest_vaas, test_sse_streaming},
    grug::NonEmpty,
    pyth_client::{middleware_cache::PythMiddlewareCache, PythClient},
    pyth_types::{
        ATOM_USD_ID, BCH_USD_ID, BNB_USD_ID, BTC_USD_ID, DOGE_USD_ID, ETH_USD_ID, LTC_USD_ID,
        PYTH_URL, SHIB_USD_ID, SOL_USD_ID, SUI_USD_ID, USDC_USD_ID, WBTC_USD_ID, XRP_USD_ID,
    },
    std::{thread::sleep, time::Duration, vec},
};

#[test]
fn write_cache() {
    let mut pyth_mock = PythMiddlewareCache::new();
    let mut pyth_client = PythClient::new(PYTH_URL);

    let ids = vec![
        ATOM_USD_ID,
        BCH_USD_ID,
        BNB_USD_ID,
        BTC_USD_ID,
        DOGE_USD_ID,
        ETH_USD_ID,
        LTC_USD_ID,
        SHIB_USD_ID,
        SOL_USD_ID,
        SUI_USD_ID,
        USDC_USD_ID,
        WBTC_USD_ID,
        XRP_USD_ID,
    ];

    for id in ids {
        // If the cache already has the data, skip.
        if pyth_mock
            .get_latest_vaas(NonEmpty::new_unchecked(vec![id]))
            .is_ok()
        {
            continue;
        }

        let mut values = vec![];
        let shared = pyth_client.run_streaming(NonEmpty::new_unchecked(vec![id]));

        while values.len() < 30 {
            let latest_vaas = shared.write_with(|mut prices_lock| {
                let prices = prices_lock.clone();
                *prices_lock = vec![];
                prices
            });

            if !latest_vaas.is_empty() {
                values.push(latest_vaas);
            }

            sleep(Duration::from_secs(2));
        }

        // Store data in the cache for each Id.
        pyth_mock.store_data(id, values).unwrap();
    }
}

#[test]
fn latest_vaas_cache() {
    let pyth_client = PythClient::new("not_real_url").with_middleware_cache();

    test_latest_vaas(pyth_client, vec![BTC_USD_ID, ETH_USD_ID]);
}

#[test]
fn sse_subscription_cache() {
    let pyth_client = PythClient::new("not_real_url").with_middleware_cache();

    test_sse_streaming(pyth_client, vec![BTC_USD_ID, ETH_USD_ID], vec![
        ATOM_USD_ID,
        BNB_USD_ID,
    ]);
}
