use {
    core::time,
    dango_testing::setup_test,
    dango_types::oracle::{
        PriceSource, QueryPriceSourcesRequest, QueryPricesRequest, ETH_USD_ID, USDC_USD_ID,
        WBTC_USD_ID,
    },
    grug::{btree_map, setup_tracing_subscriber, Denom, ResultExt},
    std::{
        str::FromStr,
        thread::{self, sleep},
        time::Duration,
    },
};

#[test]
fn proposal_pyth() {
    let (mut suite, _, _, contracts) = setup_test();

    setup_tracing_subscriber(tracing::Level::INFO);

    let price_ids = btree_map! {
        Denom::from_str("usdc").unwrap() => PriceSource::Pyth { id: USDC_USD_ID, precision: 6 },
        Denom::from_str("btc").unwrap()  => PriceSource::Pyth { id: WBTC_USD_ID, precision: 8 },
        Denom::from_str("eth").unwrap()  => PriceSource::Pyth { id: ETH_USD_ID, precision: 18 },
    };

    // Check if they are registered.
    let res = suite
        .query_wasm_smart(contracts.oracle, QueryPriceSourcesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    assert_eq!(res, price_ids);

    // Trigger the prepare proposal to write the price ids into
    // Shared pyth_ids variable.
    suite.make_empty_block();

    // Give time to the thread to write the price into
    // Shared latest_vaas variable.
    thread::sleep(time::Duration::from_secs(2));

    // Trigger the prepare proposal to upload the prices to oracle.
    suite.make_empty_block();

    // Retreive the prices.
    let prices1 = suite
        .query_wasm_smart(contracts.oracle, QueryPricesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    // Await some time and assert that the timestamp are updated.
    sleep(time::Duration::from_secs(2));

    suite.make_empty_block();

    let prices2 = suite
        .query_wasm_smart(contracts.oracle, QueryPricesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    // Assert that the timestamp are updated.
    for (denom, price) in prices1 {
        assert_ne!(price.timestamp, prices2.get(&denom).unwrap().timestamp);
    }

    // Create some empty blocks and give some time
    // to the thread to write the price into the Shared prices variable.
    for _ in 0..5 {
        suite.make_empty_block();
        thread::sleep(Duration::from_secs(2));
    }
}
