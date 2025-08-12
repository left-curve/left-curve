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

    // Assert the handler is working.
    check_handler_works(&handler, 3);

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

    // Assert the handler is working.
    check_handler_works(&handler, 3);
}

// Check the handler returns data correctly.
fn check_handler_works(handler: &PythHandler<PythClientCache>, data_wanted: usize) {
    // Assert the streaming is working.
    let mut received_data = 0;
    let mut previous_data = None;

    for _ in 0..data_wanted * 5 {
        let data = handler.fetch_latest_vaas();

        // Check if we received some data.
        if !data.is_empty() {
            received_data += 1;

            if let Some(previous) = previous_data {
                assert!(data != previous, "Data should change over time");
            }
            previous_data = Some(data);

            // Now that we have read the data, the next iteration should be empty
            // since the handler didn't have time to fetch new data.
            assert!(handler.fetch_latest_vaas().is_empty());

            // We have met the data wanted, we can stop.
            if received_data >= data_wanted {
                return;
            }
        }

        sleep(Duration::from_millis(500));
    }

    panic!("Expected to receive at least {data_wanted} data, but received only {received_data}");
}
