use {
    dango_types::oracle::PythVaa,
    grug::{btree_map, Inner, Lengthy, MockApi, NonEmpty},
    pyth_client::PythClient,
    std::{thread::sleep, time::Duration},
};

// Test the latest vaas.
pub fn test_latest_vaas<I>(mut pyth_client: PythClient, ids: I)
where
    I: IntoIterator + Clone + Lengthy,
    I::Item: ToString,
{
    let api = MockApi;

    // Latest values to asset that the prices and publish times change.
    let mut latest_values = btree_map!();
    for id in ids.clone().into_iter() {
        latest_values.insert(id.to_string(), (0, 0));
    }

    let ids = NonEmpty::new_unchecked(ids);

    for _ in 0..10 {
        let vaas = pyth_client.get_latest_vaas(ids.clone()).unwrap();

        assert!(!vaas.is_empty(), "No vaas found");

        for vaa in vaas.iter() {
            for price_feed in PythVaa::new(&api, vaa.clone().into_inner())
                .unwrap()
                .unverified()
            {
                let new_price = price_feed.get_price_unchecked().price;
                let new_publish_time = price_feed.get_price_unchecked().publish_time;

                assert_ne!(
                    new_price,
                    latest_values.get(&price_feed.id.to_string()).unwrap().0,
                    "Price has not changed"
                );
                assert!(
                    new_publish_time > latest_values.get(&price_feed.id.to_string()).unwrap().1,
                    "Time has not increased"
                );

                latest_values.insert(price_feed.id.to_string(), (new_price, new_publish_time));
            }
        }
        sleep(Duration::from_secs(2));
    }
}

// Test the sse streaming.
pub fn test_sse_streaming<I>(mut pyth_client: PythClient, ids1: I, ids2: I)
where
    I: IntoIterator + Clone + Lengthy + Send + 'static,
    I::Item: ToString,
{
    let api = MockApi;

    // Latest values to asset that the prices and publish times change.
    let mut latest_values = btree_map!();
    for id in ids1.clone().into_iter() {
        latest_values.insert(id.to_string(), (0, 0));
    }

    let shared = pyth_client.run_streaming(NonEmpty::new_unchecked(ids1.clone()));

    sleep(Duration::from_secs(1));
    for _ in 0..10 {
        // Read from the shared memory.
        let mut vaas = Vec::new();
        shared.write_with(|mut shared_vaas| {
            vaas = shared_vaas.clone();
            *shared_vaas = vec![];
        });

        assert!(!vaas.is_empty());

        let mut count_price_feed = 0;
        for vaa in vaas.iter() {
            for price_feed in PythVaa::new(&api, vaa.clone().into_inner())
                .unwrap()
                .unverified()
            {
                count_price_feed += 1;

                let new_price = price_feed.get_price_unchecked().price;
                let new_publish_time = price_feed.get_price_unchecked().publish_time;

                assert_ne!(
                    new_price,
                    latest_values.get(&price_feed.id.to_string()).unwrap().0,
                    "Price has not changed"
                );
                assert!(
                    new_publish_time > latest_values.get(&price_feed.id.to_string()).unwrap().1,
                    "Time has not increased"
                );

                latest_values.insert(price_feed.id.to_string(), (new_price, new_publish_time));
            }
        }

        assert!(count_price_feed == ids1.length(), "Not all feeds were read");

        sleep(Duration::from_secs(1));
    }

    // Close the stream.
    pyth_client.close();

    // Empty the shared memory.
    shared.write_with(|mut shared_vaas| {
        shared_vaas.clear();
    });

    // Wait some times before ensuring the stream is closed.
    sleep(Duration::from_secs(5));

    // Ensure the shared is empty.
    shared.write_with(|shared_vaas| {
        assert!(shared_vaas.is_empty());
    });

    // Run the previous test with the second set of ids.
    let mut latest_values = btree_map!();
    for id in ids2.clone().into_iter() {
        latest_values.insert(id.to_string(), (0, 0));
    }

    let shared = pyth_client.run_streaming(NonEmpty::new_unchecked(ids2.clone()));

    sleep(Duration::from_secs(1));
    for _ in 0..10 {
        // Read from the shared memory.
        let mut vaas = Vec::new();
        shared.write_with(|mut shared_vaas| {
            vaas = shared_vaas.clone();
            *shared_vaas = vec![];
        });

        assert!(!vaas.is_empty());

        let mut count_price_feed = 0;
        for vaa in vaas.iter() {
            for price_feed in PythVaa::new(&api, vaa.clone().into_inner())
                .unwrap()
                .unverified()
            {
                count_price_feed += 1;

                let new_price = price_feed.get_price_unchecked().price;
                let new_publish_time = price_feed.get_price_unchecked().publish_time;

                assert_ne!(
                    new_price,
                    latest_values.get(&price_feed.id.to_string()).unwrap().0,
                    "Price has not changed (may happened)"
                );
                assert!(
                    new_publish_time > latest_values.get(&price_feed.id.to_string()).unwrap().1,
                    "Time has not increased"
                );

                latest_values.insert(price_feed.id.to_string(), (new_price, new_publish_time));
            }
        }

        assert!(count_price_feed == ids1.length(), "Not all feeds were read");

        sleep(Duration::from_secs(1));
    }
}
