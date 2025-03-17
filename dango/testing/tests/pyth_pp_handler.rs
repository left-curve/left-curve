use {
    dango_app::PythPPHandler,
    dango_testing::setup_test,
    dango_types::oracle::InstantiateMsg,
    grug::{btree_map, Coins, HashExt, Querier, QuerierWrapper, ResultExt, StdError},
    grug_app::AppError,
    pyth_client::client_cache::PythClientCache,
    pyth_types::PYTH_URL,
    std::{thread::sleep, time::Duration},
};

struct QueryWrapperTest<'a> {
    querier: QuerierWrapper<'a, AppError>,
}

impl Querier for QueryWrapperTest<'_> {
    type Error = StdError;

    fn query_chain(&self, req: grug::Query) -> Result<grug::QueryResponse, Self::Error> {
        self.querier
            .query_chain(req)
            .map_err(|_| StdError::host("query_chain failed".to_string()))
    }
}

#[test]
fn handler() {
    let (mut suite, mut accounts, codes, contracts) = setup_test();

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
            "dada",
            None,
            None,
            Coins::new(),
        )
        .should_succeed()
        .address;

    let wrapper_test: QueryWrapperTest<'_> = QueryWrapperTest {
        querier: QuerierWrapper::new(&suite),
    };
    let querier = QuerierWrapper::new(&wrapper_test);

    let mut handler = PythPPHandler::<PythClientCache>::new_with_cache(PYTH_URL);

    // Start the handler with oracle.
    handler.update_stream(&querier, oracle).unwrap();

    // Give some times to get the data ready.
    sleep(Duration::from_millis(500));

    // Assert the streaming is working.
    for _ in 0..3 {
        assert!(!handler.fetch_latest_vaas().is_empty());
        sleep(Duration::from_millis(500));
    }

    // Update the handler with the empty oracle.
    handler.update_stream(&querier, empty_oracle).unwrap();

    // Remove possible data.
    handler.fetch_latest_vaas();

    // Assert the streaming is closed.
    for _ in 0..3 {
        assert!(handler.fetch_latest_vaas().is_empty());
        sleep(Duration::from_millis(500));
    }

    // Update the handler with oracle.
    handler.update_stream(&querier, oracle).unwrap();

    // Give some times to get the data ready.
    sleep(Duration::from_millis(500));

    // Assert the streaming is working.
    for _ in 0..3 {
        assert!(!handler.fetch_latest_vaas().is_empty());
        sleep(Duration::from_millis(500));
    }
}
