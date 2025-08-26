use {
    grug::{Inner, NonEmpty},
    pyth_client::PythClientTrait,
    pyth_lazer::PythClientLazer,
    pyth_types::{
        PayloadData,
        constants::{
            BTC_USD_ID_LAZER, DOGE_USD_ID_LAZER, ETH_USD_ID_LAZER, LAZER_ACCESS_TOKEN_TEST,
            LAZER_ENDPOINTS_TEST,
        },
    },
    std::time::Duration,
    tokio::time::sleep,
    tokio_stream::StreamExt,
};

#[tokio::test]
async fn pyth_client_lazer() {
    let pyth_ids =
        NonEmpty::new_unchecked(vec![BTC_USD_ID_LAZER, ETH_USD_ID_LAZER, DOGE_USD_ID_LAZER]);
    let num_ids = pyth_ids.len();

    let mut client = PythClientLazer::new(LAZER_ENDPOINTS_TEST, LAZER_ACCESS_TOKEN_TEST).unwrap();

    let mut stream = client.stream(pyth_ids).await.unwrap();

    let mut read_all_ids = false;
    // Read 10 items
    for _ in 0..10 {
        let data = stream.next().await;
        assert!(data.is_some());
        let mut num_feeds = 0;

        for data in data.unwrap().try_into_lazer().unwrap().inner() {
            let payload = PayloadData::deserialize_slice_le(&data.payload).unwrap();
            num_feeds += payload.feeds.len();
        }

        if num_feeds != num_ids {
            sleep(Duration::from_millis(200)).await;
        } else {
            read_all_ids = true;
        }
    }

    assert!(read_all_ids);

    // Close the stream.
    client.close();

    sleep(Duration::from_millis(100)).await;

    // Assert that the stream is closed.
    for _ in 0..10 {
        assert!(stream.next().await.is_none());
        sleep(Duration::from_millis(100)).await;
    }

    // Create a second stream.
    let pyth_ids = NonEmpty::new_unchecked(vec![BTC_USD_ID_LAZER]);
    let num_ids = pyth_ids.len();

    let mut stream = client.stream(pyth_ids).await.unwrap();

    // Read 10 items
    for _ in 0..10 {
        let data = stream.next().await;
        assert!(data.is_some());

        let payload = PayloadData::deserialize_slice_le(
            &data.unwrap().try_into_lazer().unwrap().inner()[0].payload,
        )
        .unwrap();
        assert_eq!(payload.feeds.len(), num_ids);
    }

    // Close the stream
    client.close();
}
