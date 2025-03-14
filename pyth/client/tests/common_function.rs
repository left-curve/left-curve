use {
    grug::{btree_map, Binary, Inner, Lengthy, MockApi, NonEmpty},
    pyth_client::{PythClient, PythClientTrait},
    pyth_types::PythVaa,
    std::{collections::BTreeMap, thread::sleep, time::Duration},
    tokio_stream::StreamExt,
};

struct VaasChecker {
    values: BTreeMap<String, BTreeMap<&'static str, i64>>,
}

impl VaasChecker {
    pub fn new<I>(ids: I) -> Self
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let mut values = BTreeMap::new();
        for id in ids.into_iter() {
            values.insert(
                id.to_string(),
                btree_map!(
                    "price" => 0,
                    "publish_time" => 0,
                    // Set to -1 since the first iteration will increase the counter
                    // (price and publish time are 0).
                    "price_change" => -1,
                    "publish_time_change" => -1,
                ),
            );
        }
        Self { values }
    }

    pub fn update_values(&mut self, vaas: Vec<Binary>) {
        assert!(!vaas.is_empty(), "No vaas found");

        let mut count_price_feed = 0;
        for vaa in vaas.into_iter() {
            for price_feed in PythVaa::new(&MockApi, vaa.into_inner())
                .unwrap()
                .unverified()
            {
                count_price_feed += 1;

                let new_price = price_feed.get_price_unchecked().price;
                let new_publish_time = price_feed.get_price_unchecked().publish_time;

                let element = self.values.get_mut(&price_feed.id.to_string()).unwrap();

                // Update the price and publish time.
                let old_price = element.insert("price", new_price).unwrap();
                let old_publish_time = element.insert("publish_time", new_publish_time).unwrap();

                assert!(new_publish_time >= old_publish_time, "Time has decreased");

                // Increase counter if the price has changed.
                element.entry("price_change").and_modify(|price_change| {
                    if new_price != old_price {
                        *price_change += 1;
                    }
                });

                // Increase counter if the publish time has changed.
                element
                    .entry("publish_time_change")
                    .and_modify(|publish_time_change| {
                        if new_publish_time != old_publish_time {
                            *publish_time_change += 1;
                        }
                    });
            }
        }
        // Assert there is 1 update for each id.
        assert!(
            count_price_feed == self.values.len(),
            "Not all feeds were read"
        );
    }

    pub fn assert_changes(&self) {
        // Asset that the prices and publish times have changed at least once.
        for (_, element) in self.values.iter() {
            assert!(element.get("price_change").unwrap() > &0, "No price change");
            assert!(
                element.get("publish_time_change").unwrap() > &0,
                "No publish time change"
            );
        }
    }
}

// Test the latest vaas.
pub fn test_latest_vaas<I, P>(pyth_client: P, ids: I)
where
    I: IntoIterator + Clone + Lengthy,
    I::Item: ToString,
    P: PythClientTrait + std::fmt::Debug,
    P::Error: std::fmt::Debug,
{
    let mut vaas_checker = VaasChecker::new(ids.clone());

    let ids = NonEmpty::new(ids).unwrap();

    for _ in 0..5 {
        // Retrieve the latest vaas.
        let vaas = pyth_client.get_latest_vaas(ids.clone()).unwrap();

        // Update the values with new ones.
        vaas_checker.update_values(vaas);

        sleep(Duration::from_millis(400));
    }

    // Assert that the prices and publish times have changed.
    vaas_checker.assert_changes();
}

// Test the sse streaming.
#[deprecated]
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
    for _ in 0..5 {
        // Read from the shared memory.
        let vaas = shared.replace(vec![]);

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
                    "Price has not changed (may happened with live data)"
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

    // Ensure the shared is still empty.
    shared.read_with(|shared_vaas| {
        assert!(shared_vaas.is_empty());
    });

    // Run the previous test with the second set of ids.
    let mut latest_values = btree_map!();
    for id in ids2.clone().into_iter() {
        latest_values.insert(id.to_string(), (0, 0));
    }

    let shared = pyth_client.run_streaming(NonEmpty::new_unchecked(ids2.clone()));

    // Need to wait because you don't know when the stream is connected
    sleep(Duration::from_secs(1));

    for _ in 0..5 {
        // Read from the shared memory.
        let vaas = shared.replace(vec![]);

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
                    "Price has not changed (may happened with live data)"
                );
                assert!(
                    new_publish_time > latest_values.get(&price_feed.id.to_string()).unwrap().1,
                    "Time has not increased"
                );

                latest_values.insert(price_feed.id.to_string(), (new_price, new_publish_time));
            }
        }

        assert!(count_price_feed == ids2.length(), "Not all feeds were read");

        sleep(Duration::from_secs(1));
    }
}

#[allow(dead_code)]
pub async fn test_stream<P, I>(client: P, ids: I)
where
    P: PythClientTrait + std::fmt::Debug,
    P::Error: std::fmt::Debug,
    I: IntoIterator + Clone + Lengthy + Send + 'static,
    I::Item: ToString,
{
    let mut vaas_checker = VaasChecker::new(ids.clone());

    let mut stream = client
        .stream(NonEmpty::new_unchecked(ids.clone()))
        .await
        .unwrap();

    let mut not_none_vaas = 0;
    while not_none_vaas < 5 {
        if let Some(vaas) = stream.next().await {
            not_none_vaas += 1;
            vaas_checker.update_values(vaas);
        };
    }

    // Asset that the prices and publish times have changed at least once.
    vaas_checker.assert_changes();
}
