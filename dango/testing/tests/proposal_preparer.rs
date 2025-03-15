use {
    core::time,
    dango_oracle::PRICES,
    dango_testing::setup_test,
    dango_types::oracle::{PriceSource, QueryPriceSourcesRequest},
    grug::{setup_tracing_subscriber, QuerierExt, ResultExt, StorageQuerier},
    std::{
        collections::{BTreeMap, BTreeSet},
        thread::{self, sleep},
        time::Duration,
    },
};

#[test]
fn proposal_pyth() {
    let (mut suite, _, _, contracts) = setup_test();

    setup_tracing_subscriber(tracing::Level::INFO);

    // Find all the prices that use the Pyth source.
    let pyth_ids = suite
        .query_wasm_smart(contracts.oracle, QueryPriceSourcesRequest {
            start_after: None,
            limit: None,
        })
        .should_succeed()
        .into_iter()
        .filter_map(|(_, price_source)| match price_source {
            PriceSource::Pyth { id, .. } => Some(id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    // Trigger the prepare proposal to write the price ids into
    // Shared pyth_ids variable.
    suite.make_empty_block();

    // Give time to the thread to write the price into
    // Shared latest_vaas variable.
    thread::sleep(time::Duration::from_secs(3));

    // Trigger the prepare proposal to upload the prices to oracle.
    suite.make_empty_block();

    // Retreive the prices and sequences.
    let prices1 = pyth_ids
        .iter()
        .map(|id| {
            let price = suite
                .query_wasm_path(contracts.oracle, &PRICES.path(*id))
                .should_succeed();
            (*id, price)
        })
        .collect::<BTreeMap<_, _>>();

    // Await some time and assert that the timestamp are updated.
    sleep(time::Duration::from_secs(2));

    suite.make_empty_block();

    let prices2 = pyth_ids
        .iter()
        .map(|id| {
            let price = suite
                .query_wasm_path(contracts.oracle, &PRICES.path(*id))
                .should_succeed();
            (*id, price)
        })
        .collect::<BTreeMap<_, _>>();

    // Assert that the prices have been updated.
    //
    // This means either the timestamp is newer, or the timestamp is equal but
    // the sequence is newer.
    for (id, (old_price, old_sequence)) in prices1 {
        let (new_price, new_sequence) = prices2.get(&id).unwrap();
        assert!(
            old_price.timestamp < new_price.timestamp
                || (old_price.timestamp == new_price.timestamp && old_sequence < *new_sequence)
        );
    }

    // Create some empty blocks and give some time
    // to the thread to write the price into the Shared prices variable.
    for _ in 0..5 {
        suite.make_empty_block();
        thread::sleep(Duration::from_secs(2));
    }
}
