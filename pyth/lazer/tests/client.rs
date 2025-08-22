use {
    core::num,
    grug::NonEmpty,
    pyth_client::PythClientTrait,
    pyth_lazer::PythClientLazer,
    pyth_types::{
        PayloadData,
        constants::{LAZER_ACCESS_TOKEN_TEST, LAZER_ENDPOINTS_TEST},
    },
    std::time::Duration,
    tokio::time::sleep,
    tokio_stream::StreamExt,
    tracing::{Level, info},
    tracing_subscriber::FmtSubscriber,
};

#[tokio::test]
async fn pyth_client_lazer() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set global tracing subscriber");

    let pyth_ids = NonEmpty::new_unchecked(vec![1, 2, 3]);
    let num_ids = pyth_ids.len();

    let mut client = PythClientLazer::new(LAZER_ENDPOINTS_TEST, LAZER_ACCESS_TOKEN_TEST).unwrap();

    let mut stream = client.stream(pyth_ids).await.unwrap();

    // Read 10 items
    for _ in 0..10 {
        let data = stream.next().await;
        info!("Received data: {:?}", data);
        assert!(data.is_some());
        let payload =
            PayloadData::deserialize_slice_le(&data.unwrap().try_into_lazer().unwrap().payload)
                .unwrap();
        assert_eq!(payload.feeds.len(), num_ids);
    }

    // Close the stream.
    client.close();

    sleep(Duration::from_millis(100)).await;

    // Assert that the stream is closed.
    for _ in 0..10 {
        assert!(stream.next().await.is_none());
        sleep(Duration::from_millis(100)).await;
    }

    // Create a second stream.
    let pyth_ids = NonEmpty::new_unchecked(vec![1]);
    let num_ids = pyth_ids.len();

    let mut stream = client.stream(pyth_ids).await.unwrap();

    // Read 10 items
    for _ in 0..10 {
        let data = stream.next().await;
        info!("Received data: {:?}", data);
        assert!(data.is_some());

        let payload =
            PayloadData::deserialize_slice_le(&data.unwrap().try_into_lazer().unwrap().payload)
                .unwrap();
        assert_eq!(payload.feeds.len(), num_ids);
    }

    // Close the stream
    client.close();
}
