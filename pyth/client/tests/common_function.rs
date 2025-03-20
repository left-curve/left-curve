use {
    grug::{Binary, Inner, Lengthy, MockApi, NonEmpty, btree_map},
    pyth_client::PythClientTrait,
    pyth_types::PythVaa,
    std::{collections::BTreeMap, fmt::Debug, thread::sleep, time::Duration},
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
            values.insert(id.to_string(), btree_map! {
                "price" => 0,
                "publish_time" => 0,
                "sequence" => 0,
                // Set to -1 since the first iteration will increase the counter
                // (price and publish time are 0).
                "price_change" => -1,
                "publish_time_change" => -1,
            });
        }
        Self { values }
    }

    pub fn update_values(&mut self, vaas: Vec<Binary>) {
        assert!(!vaas.is_empty(), "No vaas found");

        let mut count_price_feed = 0;
        for vaa in vaas.into_iter() {
            let pyth_vaa = PythVaa::new(&MockApi, vaa.into_inner()).unwrap();
            let new_sequence = pyth_vaa.wormhole_vaa.sequence as i64;

            for price_feed in pyth_vaa.unverified() {
                count_price_feed += 1;

                let new_price = price_feed.get_price_unchecked().price;
                let new_publish_time = price_feed.get_price_unchecked().publish_time;

                let element = self.values.get_mut(&price_feed.id.to_string()).unwrap();

                // Update the price and publish time.
                let old_price = element.insert("price", new_price).unwrap();
                let old_publish_time = element.insert("publish_time", new_publish_time).unwrap();
                let old_sequence = element.insert("sequence", new_sequence).unwrap();

                assert!(new_publish_time >= old_publish_time, "Time has decreased");
                assert!(new_sequence > old_sequence, "Sequence has decreased");

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
pub fn test_latest_vaas<P, I>(pyth_client: P, ids: I)
where
    P: PythClientTrait + Debug,
    P::Error: Debug,
    I: IntoIterator + Clone + Lengthy,
    I::Item: ToString,
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

// Test for streaming vaas.
pub async fn test_stream<P, I>(mut client: P, ids1: I, ids2: I)
where
    P: PythClientTrait + Debug,
    P::Error: Debug,
    I: IntoIterator + Clone + Lengthy + Send + 'static,
    I::Item: ToString,
{
    let mut vaas_checker = VaasChecker::new(ids1.clone());
    let mut stream = client.stream(NonEmpty::new_unchecked(ids1)).await.unwrap();
    let mut not_none_vaas = 0;

    while not_none_vaas < 5 {
        if let Some(vaas) = stream.next().await {
            not_none_vaas += 1;
            vaas_checker.update_values(vaas);
        }
    }

    // Asset that the prices and publish times have changed at least once.
    vaas_checker.assert_changes();

    // Close the stream.
    client.close();

    // Read the stream; there could be one remaining value.
    stream.next().await;

    // Assert that the stream is closed.
    for _ in 0..5 {
        assert!(stream.next().await.is_none(), "Stream is not closed");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Open a new connection with the second ids.
    let mut vaas_checker = VaasChecker::new(ids2.clone());
    let mut stream = client.stream(NonEmpty::new_unchecked(ids2)).await.unwrap();
    let mut not_none_vaas = 0;

    while not_none_vaas < 5 {
        if let Some(vaas) = stream.next().await {
            not_none_vaas += 1;
            vaas_checker.update_values(vaas);
        }
    }

    // Asset that the prices and publish times have changed at least once.
    vaas_checker.assert_changes();

    // Close the stream.
    client.close();

    // Read the stream; there could be one remaining value.
    stream.next().await;

    // Assert that the stream is closed.
    for _ in 0..5 {
        assert!(stream.next().await.is_none(), "Stream is not closed");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
