use {
    dango_types::oracle::PythVaa,
    grug::{Inner, MockApi, NonEmpty},
    pyth_client::PythClient,
    pyth_types::{BTC_USD_ID, ETH_USD_ID, PYTH_URL},
    std::time::Duration,
    tokio::time::sleep,
};

#[tokio::test]
async fn test_client() {
    let api = MockApi;

    let mut client = PythClient::new(PYTH_URL);
    let ids = vec![
        ("ids[]", BTC_USD_ID.to_string()),
        ("ids[]", ETH_USD_ID.to_string()),
    ];

    let shared = client
        .run_streaming(NonEmpty::new(ids).unwrap(), None)
        .unwrap();

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
}
