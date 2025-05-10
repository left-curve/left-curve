use {
    dango_proposal_preparer::PythHandler,
    dango_testing::setup_test,
    dango_types::oracle::{InstantiateMsg, PriceSource, QueryPriceSourcesRequest},
    grug::{Coins, HashExt, NonEmpty, QuerierExt, QuerierWrapper, ResultExt, btree_map},
    pyth_client::{PythClientCache, PythClientTrait},
    pyth_types::constants::PYTH_URL,
    std::{thread::sleep, time::Duration},
};

#[test]
fn handler() {
    let (mut suite, mut accounts, codes, contracts, _) = setup_test(Default::default());

    // Ensure there are all cache file for the PythIds in oracle and retrieve them if not presents.
    // This is needed since the PythPPHandler create a thread to get the data from Pyth and if the
    // cache files are not present the thread will not wait for client to retrieve
    // and save them. The test will end before the client is able to finish.
    {
        // Retrieve all PythIds from the oracle.
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
            .collect::<Vec<_>>();

        // Create cache for ids if not present.
        PythClientCache::new(PYTH_URL)
            .unwrap()
            .get_latest_vaas(NonEmpty::new(pyth_ids).unwrap())
            .unwrap();
    }

    // Oracle from the setup_test has some PythIds already uploaded.
    let oracle = contracts.oracle;

    // Create an empty oracle (without any PythIds) to test the handler correctly
    // close the streaming.
    let empty_oracle = suite
        .instantiate(
            &mut accounts.owner,
            codes.oracle.to_bytes().hash256(),
            &InstantiateMsg {
                guardian_sets: btree_map!(),
                price_sources: btree_map!(),
            },
            "salt",
            None,
            None,
            Coins::new(),
        )
        .should_succeed()
        .address;

    let querier = QuerierWrapper::new(&suite);
    let mut handler = PythHandler::<PythClientCache>::new_with_cache(PYTH_URL);

    // Start the handler with oracle.
    handler.update_stream(querier, oracle).unwrap();

    // Give some times to get the data ready.
    sleep(Duration::from_millis(500));

    // Assert the streaming is working.
    for _ in 0..3 {
        assert!(!handler.fetch_latest_vaas().is_empty());
        sleep(Duration::from_millis(500));
    }

    // Update the handler with the empty oracle.
    handler.update_stream(querier, empty_oracle).unwrap();

    // Remove possible data.
    handler.fetch_latest_vaas();

    // Assert the streaming is closed.
    for _ in 0..3 {
        assert!(handler.fetch_latest_vaas().is_empty());
        sleep(Duration::from_millis(500));
    }

    // Update the handler with oracle.
    handler.update_stream(querier, oracle).unwrap();

    // Give some times to get the data ready.
    sleep(Duration::from_millis(500));

    // Assert the streaming is working.
    for _ in 0..3 {
        assert!(!handler.fetch_latest_vaas().is_empty());
        sleep(Duration::from_millis(500));
    }
}
