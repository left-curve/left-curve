use {
    dango_types::oracle::PythVaa,
    grug::{Inner, MockApi, NonEmpty},
    pyth_client::PythClient,
    pyth_types::{BTC_USD_ID, ETH_USD_ID},
    std::{thread::sleep, time::Duration},
};

#[test]
fn middleware_sse() {
    let mut pyth_client = PythClient::new("not_real_url").with_middleware_cache();
    let api = MockApi;

    let shared = pyth_client.run_streaming(NonEmpty::new_unchecked(vec![BTC_USD_ID, ETH_USD_ID]));

    sleep(Duration::from_secs(1));
    for _ in 0..5 {
        // Read from the shared memory.
        let mut vaas = Vec::new();
        shared.write_with(|mut shared_vaas| {
            vaas = shared_vaas.clone();
            *shared_vaas = vec![];
        });

        assert!(!vaas.is_empty());

        // Check the vaas.
        for vaa in vaas {
            let vaa = PythVaa::new(&api, vaa.into_inner()).unwrap();
            for feed in vaa.unverified() {
                println!("feed: {:?}", feed);
            }
        }
        sleep(Duration::from_secs(1));
    }
}
