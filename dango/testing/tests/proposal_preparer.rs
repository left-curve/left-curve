use {
    core::time,
    dango_testing::setup_test,
    dango_types::oracle::{
        ExecuteMsg, PriceSource, QueryPriceSourcesRequest, QueryPricesRequest, ETH_USD_ID,
        USDC_USD_ID, WBTC_USD_ID,
    },
    grug::{btree_map, Coins, Denom, ResultExt},
    std::{str::FromStr, thread::sleep},
};

#[test]
fn proposal_pyth() {
    let (mut suite, mut account, _, contracts) = setup_test();

    let price_ids = btree_map! {
        Denom::from_str("usdc").unwrap() => PriceSource::Pyth { id: USDC_USD_ID, precision: 6 },
        Denom::from_str("btc").unwrap()  => PriceSource::Pyth { id: WBTC_USD_ID, precision: 8 },
        Denom::from_str("eth").unwrap()  => PriceSource::Pyth { id: ETH_USD_ID, precision: 18 },
    };

    // Register the price sources.
    suite
        .execute(
            &mut account.owner,
            contracts.oracle,
            &ExecuteMsg::RegisterPriceSources(price_ids.clone()),
            Coins::default(),
        )
        .should_succeed();

    // Check if they are registered.
    let res = suite
        .query_wasm_smart(contracts.oracle, QueryPriceSourcesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    assert_eq!(res, price_ids);

    // Trigger the prepare proposal to update the prices.
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
}
