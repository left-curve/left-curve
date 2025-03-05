use {
    dango_types::oracle::PythVaa,
    grug::{Inner, MockApi, NonEmpty},
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

    // Set the ids group to store.
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

        while values.len() < 20 {
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
        pyth_mock.store_data(vec![id], values).unwrap();
    }
}

#[test]
fn read_single_cache() {
    let api = MockApi;
    let mut pyth_client = PythClient::new("not_real_url").with_middleware_cache();

    // Read the stored files.
    let mut latest_time = 0;
    let mut latest_price = 0;
    for _ in 0..10 {
        let vaas = pyth_client
            .get_latest_vaas(NonEmpty::new_unchecked(vec![BTC_USD_ID]))
            .unwrap();

        for vaa in vaas {
            let vaa = PythVaa::new(&api, vaa.into_inner()).unwrap();
            for feed in vaa.unverified() {
                let new_price = feed.get_price_unchecked().price;
                let new_publish_time = feed.get_price_unchecked().publish_time;

                assert_ne!(new_price, latest_price, "Price has not changed");
                assert!(new_publish_time > latest_time, "Time has not increased");

                latest_price = new_price;
                latest_time = new_publish_time;

                println!("price: {:?}, time {:?}", new_price, new_publish_time);
            }
        }
    }
}

#[test]
fn read_multiple_cache() {
    let api = MockApi;
    let mut pyth_client = PythClient::new("not_real_url").with_middleware_cache();

    // Read the stored files.
    let mut latest_price_btc = 0;
    let mut latest_time_btc = 0;

    let mut latest_price_eth = 0;
    let mut latest_time_eth = 0;

    for _ in 0..10 {
        let vaas = pyth_client
            .get_latest_vaas(NonEmpty::new_unchecked(vec![BTC_USD_ID, ETH_USD_ID]))
            .unwrap();

        let btc_price_feed = *PythVaa::new(&api, vaas.first().unwrap().clone().into_inner())
            .unwrap()
            .unverified()
            .first()
            .unwrap();

        let new_price_btc = btc_price_feed.get_price_unchecked().price;
        let new_publish_time_btc = btc_price_feed.get_price_unchecked().publish_time;

        assert_ne!(new_price_btc, latest_price_btc, "Price has not changed");
        assert!(
            new_publish_time_btc > latest_time_btc,
            "Time has not increased"
        );

        latest_price_btc = new_price_btc;
        latest_time_btc = new_publish_time_btc;

        println!(
            "BTC price: {:?}, time {:?}",
            new_price_btc, new_publish_time_btc
        );

        // eth
        let eth_price_feed = *PythVaa::new(&api, vaas.get(1).unwrap().clone().into_inner())
            .unwrap()
            .unverified()
            .first()
            .unwrap();

        let new_price_eth = eth_price_feed.get_price_unchecked().price;
        let new_publish_time_eth = eth_price_feed.get_price_unchecked().publish_time;

        assert_ne!(new_price_eth, latest_price_eth, "Price has not changed");
        assert!(
            new_publish_time_eth > latest_time_eth,
            "Time has not increased"
        );

        latest_price_eth = new_price_eth;
        latest_time_eth = new_publish_time_eth;

        println!(
            "ETH price: {:?}, time {:?}",
            new_price_eth, new_publish_time_eth
        );
    }
}
