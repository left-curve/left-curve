use {
    dango_oracle::PRICES,
    dango_testing::setup_test,
    dango_types::oracle::{ExecuteMsg, PriceSource, QueryPriceRequest, QueryPriceSourcesRequest},
    grug::{
        Coins, Denom, NonEmpty, QuerierExt, ResultExt, StorageQuerier, btree_map,
        setup_tracing_subscriber,
    },
    hex_literal::hex,
    pyth_client::{PythClientCache, PythClientTrait},
    pyth_types::{PYTH_URL, PythId},
    std::{
        collections::{BTreeMap, BTreeSet},
        str::FromStr,
        thread::{self, sleep},
        time::Duration,
    },
};

const NOT_USED_ID: PythId = PythId::from_inner(hex!(
    "2b9ab1e972a281585084148ba1389800799bd4be63b957507db1349314e47445"
));

#[test]
fn proposal_pyth() {
    // Ensure there are all cache file for the PythIds in oracle and also for
    // the NOT_USED_ID and retrieve them if not presents. This is needed since
    // the PythPPHandler create a thread to get the data from Pyth and if the
    // cache files are not present the thread will not wait for client to retrieve
    // and save them. The test will end before the client is able to finish.
    {
        let (suite, _, _, contracts) = setup_test();

        // Retrieve all PythIds from the oracle.
        let mut pyth_ids = suite
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
            .collect::<Vec<_>>();

        // Create cache for ids if not present.
        pyth_ids.push(NOT_USED_ID);
        PythClientCache::new(PYTH_URL)
            .unwrap()
            .get_latest_vaas(NonEmpty::new(pyth_ids).unwrap())
            .unwrap();
    }

    let (mut suite, mut accounts, _, contracts) = setup_test();
    setup_tracing_subscriber(tracing::Level::DEBUG);

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
    thread::sleep(Duration::from_secs(1));

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
    sleep(Duration::from_secs(2));

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

    // Push a new PythId to oracle to verify that the handler update the
    // ids correctly.
    {
        let test_denom = Denom::from_str("test").unwrap();

        // Verify the denom does not exist in the oracle.
        suite
            .query_wasm_smart(contracts.oracle, QueryPriceRequest {
                denom: test_denom.clone(),
            })
            .should_fail_with_error("data not found");

        // Verify the NOT_USED_ID is not in the oracle.
        let _ = suite
            .query_wasm_smart(contracts.oracle, QueryPriceSourcesRequest {
                start_after: None,
                limit: Some(u32::MAX),
            })
            .should_succeed()
            .values()
            .map(|price_source| {
                if let PriceSource::Pyth { id, .. } = price_source {
                    assert_ne!(id, NOT_USED_ID);
                }
            });

        // Push NOT_USED_ID to the oracle.
        let msg =
            ExecuteMsg::RegisterPriceSources(btree_map!( test_denom.clone() => PriceSource::Pyth {
                id: NOT_USED_ID,
                precision: 6,
            }));

        suite
            .execute(&mut accounts.owner, contracts.oracle, &msg, Coins::new())
            .should_succeed();

        // Run few blocks to trigger the prepare proposal to update the ids
        // and upload the prices.
        suite.make_empty_block();
        thread::sleep(Duration::from_secs(1));
        suite.make_empty_block();

        // Verify that the price exists.
        suite
            .query_wasm_smart(contracts.oracle, QueryPriceRequest { denom: test_denom })
            .should_succeed();
    }
}
