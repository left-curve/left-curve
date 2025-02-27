use {
    dango_types::oracle::PythVaa,
    grug::{Inner, MockApi},
    pyth::{PythClient, PYTH_URL},
    std::time::Duration,
    tokio::time::sleep,
};

#[tokio::test]
async fn test_client() {
    let api = MockApi;

    let mut client = PythClient::new(PYTH_URL);
    let ids = vec![
        (
            "ids[]",
            "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
        ),
        (
            "ids[]",
            "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace".to_string(),
        ),
    ];

    let mut rx = client.run_streaming(ids).unwrap();

    for _ in 0..10 {
        if let Some(vaas) = rx.recv().await {
            println!("New message");
            // Decode the binary data
            for vaa in vaas.binary.data {
                let vaa = PythVaa::new(&api, vaa.into_inner()).unwrap();
                for feed in vaa.unverified() {
                    println!("feed: {:?}", feed);
                }
            }
            println!();
        } else {
            sleep(Duration::from_secs(1)).await;
        }
    }
}
