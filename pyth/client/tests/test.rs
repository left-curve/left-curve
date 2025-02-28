use {
    dango_types::oracle::PythVaa,
    grug::{setup_tracing_subscriber, Inner, MockApi, NonEmpty},
    pyth_client::PythClient,
    pyth_types::{ATOM_USD_ID, BTC_USD_ID, ETH_USD_ID, PYTH_URL},
    std::time::Duration,
    tokio::time::sleep,
};

#[tokio::test]
async fn test_client() {
    setup_tracing_subscriber(tracing::Level::INFO);

    let api = MockApi;

    let mut client = PythClient::new(PYTH_URL);
    let ids = NonEmpty::new(vec![BTC_USD_ID, ETH_USD_ID]).unwrap();

    let shared = client.run_streaming(ids, None);

    for _ in 0..2 {
        for _ in 0..10 {
            // Read the vaas from the shared memory.
            let mut vaas = Vec::new();
            shared.write_with(|mut shared_vaas| {
                vaas = shared_vaas.clone();
                *shared_vaas = vec![];
            });

            if vaas.is_empty() {
                sleep(Duration::from_secs(1)).await;
            } else {
                for vaa in vaas {
                    let vaa = PythVaa::new(&api, vaa.into_inner()).unwrap();
                    for feed in vaa.unverified() {
                        println!("feed: {:?}", feed);
                    }
                }
            }
        }

        // Close the connection, adding a new id and restarting the connection.
        client.close();
        let ids = NonEmpty::new(vec![BTC_USD_ID, ETH_USD_ID, ATOM_USD_ID]).unwrap();
        client.run_streaming(ids, Some(shared.clone()));
    }
}
