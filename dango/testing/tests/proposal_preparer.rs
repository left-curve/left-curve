use {
    dango_testing::setup_test,
    dango_types::{self, oracle::QueryPricesRequest},
    grug::{setup_tracing_subscriber, QuerierExt, ResultExt},
    std::{
        thread::{self, sleep},
        time::Duration,
    },
};

#[test]
fn proposal_pyth() {
    setup_tracing_subscriber(tracing::Level::DEBUG);
    let (mut suite, _, _, contracts) = setup_test();

    // Trigger the prepare proposal to write the price ids into
    // Shared pyth_ids variable.
    suite.make_empty_block();

    // Give time to the thread to write the price into
    // Shared latest_vaas variable.
    thread::sleep(Duration::from_secs(1));

    // Trigger the prepare proposal to upload the prices to oracle.
    suite.make_empty_block();

    // Retrieve the prices.
    let prices1 = suite
        .query_wasm_smart(contracts.oracle, QueryPricesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    // Await some time and assert that the timestamp are updated.
    sleep(Duration::from_secs(2));

    suite.make_empty_block();

    let prices2 = suite
        .query_wasm_smart(contracts.oracle, QueryPricesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed();

    // Assert that the timestamp are updated.
    for (denom, price) in prices1 {
        assert!(
            price.timestamp < prices2.get(&denom).unwrap().timestamp,
            "Timestamp not updated"
        );
    }

    // // Create some empty blocks and give some time
    // // to the thread to write the price into the Shared prices variable.
    // for _ in 0..5 {
    //     suite.make_empty_block();
    //     thread::sleep(Duration::from_secs(2));
    // }
}
