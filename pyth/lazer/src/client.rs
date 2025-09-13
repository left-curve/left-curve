use {
    anyhow::bail,
    async_stream::stream,
    chrono::{Local, TimeDelta},
    grug::{Inner, Lengthy, NonEmpty},
    pyth_client::PythClientTrait,
    pyth_lazer_client::{
        client::{PythLazerClient, PythLazerClientBuilder},
        ws_connection::AnyResponse,
    },
    pyth_lazer_protocol::{
        message::Message,
        router::{
            Channel, DeliveryFormat, Format, JsonBinaryEncoding, PriceFeedId, PriceFeedProperty,
            SubscriptionParams, SubscriptionParamsRepr,
        },
        subscription::{Response, SubscribeRequest, SubscriptionId},
    },
    pyth_types::{PriceUpdate, PythLazerSubscriptionDetails},
    reqwest::IntoUrl,
    std::{
        collections::HashMap,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    },
    tokio::{sync::mpsc::Receiver, time::sleep},
    tracing::{debug, error, info, warn},
    url::Url,
};

pub const RESUBSCRIBE_ATTEMPTS: usize = 5;

#[derive(Clone, Debug)]
pub struct PythClientLazer {
    endpoints: Vec<Url>,
    access_token: String,
    keep_running: Arc<AtomicBool>,
    last_subscription_id: u64,
}

impl PythClientLazer {
    pub fn new<V, U, T>(endpoints: NonEmpty<V>, access_token: T) -> Result<Self, anyhow::Error>
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        Ok(PythClientLazer {
            endpoints: endpoints
                .into_inner()
                .into_iter()
                .map(|url| url.into_url())
                .collect::<Result<Vec<_>, _>>()?,
            access_token: access_token.to_string(),
            keep_running: Arc::new(AtomicBool::new(false)),
            last_subscription_id: 0,
        })
    }

    /// Create subscription parameters for the given price feed IDs.
    fn subscription_params(
        price_feed_ids: Vec<PriceFeedId>,
        channel: Channel,
    ) -> Result<SubscriptionParams, anyhow::Error> {
        SubscriptionParams::new(SubscriptionParamsRepr {
            price_feed_ids,
            properties: vec![PriceFeedProperty::Price, PriceFeedProperty::Exponent],
            formats: vec![Format::LeEcdsa],
            delivery_format: DeliveryFormat::Binary,
            json_binary_encoding: JsonBinaryEncoding::Base64,
            parsed: false,
            channel,
            ignore_invalid_feed_ids: true,
        })
        .map_err(|e| anyhow::anyhow!(e))
    }

    // Subscribe to the price feeds.
    // The subscribe function return error if at least one subscription fails.
    // Ignore the error here and analyze the data received from the stream: in case
    // we don't receive any data for a subscription, we will resubscribe.
    async fn connect(client: &mut PythLazerClient, subscribe_requests: Vec<SubscribeRequest>) {
        info!(
            "Sending subscription with {} requests",
            subscribe_requests.len()
        );
        for subscription in subscribe_requests {
            let _ = client.subscribe(subscription).await;
        }
    }

    /// Subscribe to the price feeds and ensure that we are receiving data for all subscriptions.
    async fn subscribe(
        client: &mut PythLazerClient,
        receiver: &mut Receiver<AnyResponse>,
        ids_per_channel: HashMap<Channel, (u64, Vec<PriceFeedId>)>,
    ) -> Result<(), anyhow::Error> {
        // Collect all subscription IDs in a vector to easily check if we are receiving data for all subscriptions.
        let mut subscription_ids = Vec::with_capacity(ids_per_channel.len());

        // Create the subscribe requests.
        let subscribe_requests = ids_per_channel
            .into_iter()
            .map(|(channel, (subscribe_id, feed_ids))| {
                let params = Self::subscription_params(feed_ids, channel)?;

                subscription_ids.push(subscribe_id);

                Ok(SubscribeRequest {
                    subscription_id: SubscriptionId(subscribe_id),
                    params,
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;

        // Subscribe to the price feeds.
        Self::connect(client, subscribe_requests.clone()).await;

        // Ensure that we are receiving data for all subscriptions.
        let mut data_per_ids = HashMap::new();
        for id in subscription_ids.clone() {
            data_per_ids.insert(id, false);
        }

        let mut resubscribe_attempts = 0;
        let mut to_resubscribe = false;
        let mut last_check = Local::now();

        let buffer_capacity = 1000;
        let mut buffer = Vec::with_capacity(buffer_capacity);
        let timeout = Duration::from_secs(3);

        loop {
            // If after few seconds we haven't received data for all subscriptions,
            // try to reconnect.
            if Local::now() - last_check > TimeDelta::seconds(2) {
                warn!("Not all subscriptions received data");
                to_resubscribe = true;
            }

            // Check if we need to resubscribe.
            if to_resubscribe {
                to_resubscribe = false;
                resubscribe_attempts += 1;

                // Return error if we have reached the maximum number of resubscription attempts.
                if resubscribe_attempts > RESUBSCRIBE_ATTEMPTS {
                    return Err(anyhow::anyhow!(
                        "Failed to connect to Pyth Lazer after {resubscribe_attempts} attempts"
                    ));
                }

                // Reset all received data to false for each subscription ID, just to be sure that
                // everything works fine.
                for (_, value) in data_per_ids.iter_mut() {
                    *value = false;
                }

                warn!(
                    "Attempting to resubscribe... (attempt {}/{})",
                    resubscribe_attempts, RESUBSCRIBE_ATTEMPTS
                );

                sleep(Duration::from_millis(100)).await;
                Self::connect(client, subscribe_requests.clone()).await;

                last_check = Local::now();
            }

            // Retrieve the data.
            buffer.clear();
            // Retrieve the data. If no data is received, try to resubscribe.
            let Some(data_count) =
                Self::retrieve_data(receiver, &mut buffer, buffer_capacity, timeout).await
            else {
                to_resubscribe = true;
                continue;
            };

            // If the number of data received is zero, it means the channel is closed and we need to resubscribe.
            if data_count == 0 {
                // No data received, continue to the next iteration to check for resubscription.
                error!("Pyth Lazer connection closed");
                to_resubscribe = true;
                continue;
            }

            debug!("Received {} messages from Pyth Lazer", data_count);

            for data in buffer.drain(..) {
                match data {
                    AnyResponse::Binary(update) => {
                        // We have received data for this subscription ID.
                        if let Some(entry) = data_per_ids.get_mut(&update.subscription_id.0) {
                            *entry = true;

                            // Check if we have received data for all subscription IDs.
                            if data_per_ids.values().all(|&v| v) {
                                info!("Successfully subscribed to all price feeds");
                                return Ok(());
                            }
                        }
                    },

                    AnyResponse::Json(response) => match response {
                        Response::Error(error_response) => {
                            error!("Subscription failed: {}", error_response.error);

                            // In this error there is no information about which subscription failed.
                            // So we will try to resubscribe to all subscriptions.
                            // If a connection is already established, the server will send a
                            // SubscriptionError response.
                            to_resubscribe = true;
                        },
                        Response::SubscriptionError(subscription_error_response) => {
                            // Ignore duplicate subscription ID errors.
                            if subscription_error_response.error != "duplicate subscription id" {
                                error!(
                                    "Subscription error for id {}: {}",
                                    subscription_error_response.subscription_id.0,
                                    subscription_error_response.error
                                );
                            }
                        },
                        Response::Subscribed(subscription_response) => {
                            info!(
                                "Subscribed with ID: {}",
                                subscription_response.subscription_id.0
                            );
                        },
                        Response::SubscribedWithInvalidFeedIdsIgnored(subscription_response) => {
                            // Log a warning if some feed ids were ignored
                            // (this means the Id or the combination Id - channel is not supported).
                            if subscription_response
                                .ignored_invalid_feed_ids
                                .unknown_ids
                                .is_empty()
                            {
                                info!(
                                    "Subscribed with ID: {}",
                                    subscription_response.subscription_id.0
                                );
                            } else {
                                warn!(
                                    "Subscribed with ID: {}, but some feed ids were ignored: {:#?}",
                                    subscription_response.subscription_id.0,
                                    subscription_response.ignored_invalid_feed_ids
                                );
                            }
                        },
                        Response::Unsubscribed(unsubscribed_response) => {
                            warn!(
                                "Unsubscribed with ID: {}",
                                unsubscribed_response.subscription_id.0
                            );
                        },

                        Response::StreamUpdated(_) => {
                            error!("Received Lazer data in json format, only support Binary");
                        },
                    },
                }
            }
        }
    }

    /// Analyze the data received from the Pyth Lazer stream.
    fn analyze_data(
        received_data: &mut Vec<AnyResponse>,
        subscription_ids: &Vec<u64>,
        subscriptions_data: &mut HashMap<u64, pyth_types::LeEcdsaMessage>,
        last_data_received: &mut HashMap<u64, chrono::DateTime<Local>>,
    ) {
        for data in received_data.drain(..) {
            match data {
                AnyResponse::Binary(update) => {
                    // Check the update is the current subscription ID.
                    if !subscription_ids.contains(&update.subscription_id.0) {
                        warn!(
                            "Received update for a different subscription ID: {}. Expected: {:?}",
                            update.subscription_id.0, subscription_ids
                        );
                    } else {
                        // Update the last time we received data for this subscription ID.
                        last_data_received.insert(update.subscription_id.0, Local::now());
                    }

                    // Analyze the messages received.
                    for msg in update.messages {
                        match msg {
                            Message::LeEcdsa(le_ecdsa_message) => {
                                // Since there are multiple subscriptions, we need to keep the last data for each subscription.
                                subscriptions_data
                                    .insert(update.subscription_id.0, le_ecdsa_message.into());
                            },
                            _ => {
                                error!("Received non-ECDSA message: {:#?}", msg);
                            },
                        }
                    }
                },

                AnyResponse::Json(response) => match response {
                    Response::Error(error_response) => {
                        error!("Received error: {:#?}", error_response);
                    },

                    Response::StreamUpdated(_) => {
                        error!("Received Lazer data in json format, only support Binary");
                    },
                    _ => {
                        warn!("Received json response: {:#?}", response);
                    },
                },
            }
        }
    }

    /// Retrieve data from the Pyth Lazer client.
    /// If no data is received within the timeout, log a warning and return None.
    /// If data is received, return the number of data received.
    async fn retrieve_data(
        receiver: &mut Receiver<AnyResponse>,
        buffer: &mut Vec<AnyResponse>,
        buffer_capacity: usize,
        timeout: Duration,
    ) -> Option<usize> {
        tokio::select! {
            // The server is not sending any more data.
            _ = tokio::time::sleep(timeout) => {
                warn!("No new data received for {} milliseconds", timeout.as_millis());
                None
            },

            // Read next data from stream.
            data_count = receiver.recv_many(buffer, buffer_capacity) => {
                Some(data_count)
            }
        }
    }
}

#[async_trait::async_trait]
impl PythClientTrait for PythClientLazer {
    type Error = anyhow::Error;
    type PythId = PythLazerSubscriptionDetails;

    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = PriceUpdate> + Send>>, Self::Error>
    where
        I: IntoIterator<Item = PythLazerSubscriptionDetails> + Lengthy + Send + Clone,
    {
        // Close the previous connection.
        self.close();

        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running = self.keep_running.clone();

        // Divide the ids depending on the channel.
        let mut ids_per_channel: HashMap<Channel, (u64, Vec<PriceFeedId>)> = HashMap::new();

        let mut subscription_ids = vec![];

        for value in ids.into_inner() {
            let channel = value.channel;

            match ids_per_channel.get_mut(&channel) {
                Some((_, price_feed)) => price_feed.push(PriceFeedId(value.id)),
                None => {
                    self.last_subscription_id += 1;

                    subscription_ids.push(self.last_subscription_id);

                    ids_per_channel.insert(
                        channel,
                        (self.last_subscription_id, vec![PriceFeedId(value.id)]),
                    );
                },
            }
        }

        // Build the new client and subscribe to the price feeds.
        let builder = PythLazerClientBuilder::new(self.access_token.clone())
            .with_endpoints(self.endpoints.clone())
            .with_timeout(Duration::from_secs(2));

        let mut client = builder.build()?;
        let mut receiver = client.start().await?;

        Self::subscribe(&mut client, &mut receiver, ids_per_channel.clone()).await?;

        // Since there are multiple subscriptions, we need to keep the last data for each subscriptions.
        let mut subscriptions_data = HashMap::with_capacity(ids_per_channel.len());

        // Create the buffer to pull data from the receiver.
        let buffer_capacity = 1000;
        let mut buffer = Vec::with_capacity(buffer_capacity);
        let timeout = Duration::from_millis(500);

        // Flag to indicate if we need to resubscribe.
        let mut to_resubscribe = false;

        // Keep track of the last time we received data for each subscription ID.
        let mut last_data_received = subscription_ids
            .iter()
            .map(|id| (*id, Local::now()))
            .collect::<HashMap<_, _>>();

        // Create the stream.
        let stream = stream! {
            loop {
                // Check if the streaming has to be closed.
                if !keep_running.load(Ordering::Acquire) {
                    info!("Pyth Lazer connection closed");
                    break;
                }

                // Check if we need to resubscribe.
                if to_resubscribe {
                    to_resubscribe = false;

                    match Self::subscribe(&mut client, &mut receiver, ids_per_channel.clone()).await
                    {
                        Ok(()) => {},
                        Err(err) => {
                            // If the subscription fails, wait for a while and try again.
                            error!("Failed to reconnect: {}", err.to_string());
                            sleep(Duration::from_millis(200)).await;
                            continue;
                        },
                    }
                }

                // Retrieve the data. If no data is received, try to resubscribe.
                buffer.clear();
                let Some(data_count) =
                    Self::retrieve_data(&mut receiver, &mut buffer, buffer_capacity, timeout).await
                else {
                    to_resubscribe = true;
                    continue;
                };

                // Connection closed, try to reconnect.
                if data_count == 0 {
                    error!("Pyth Lazer connection closed");
                    to_resubscribe = true;
                    continue;
                };

                // Analyze the data received.
                Self::analyze_data(
                    &mut buffer,
                    &subscription_ids,
                    &mut subscriptions_data,
                    &mut last_data_received,
                );

                // Check if we haven't received data for some subscriptions for more than 3 seconds.
                let now = Local::now();
                for (id, last_time) in last_data_received.iter() {
                    if now - *last_time > TimeDelta::seconds(3) {
                        to_resubscribe = true;
                        warn!("No data received for subscription ID {id} for more than 3 seconds");
                    }
                }

                // Yield the current data.
                if !subscriptions_data.is_empty() {
                    let send_data = subscriptions_data.clone();
                    yield PriceUpdate::Lazer(NonEmpty::new_unchecked(
                        send_data.into_values().collect::<Vec<_>>(),
                    ));
                }
            }

            // If the code reaches here, it means the stream needs to be closed.
            for id in subscription_ids {
                match client.unsubscribe(SubscriptionId(id)).await {
                    Ok(_) => {
                        info!("Unsubscribed stream id {} successfully", id);
                    },
                    Err(e) => {
                        error!("Failed to unsubscribe stream id {}: {:?}", id, e);
                    },
                };
            }
        };

        Ok(Box::pin(stream))
    }

    fn get_latest_price_update<I>(&self, _ids: NonEmpty<I>) -> Result<PriceUpdate, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        // TODO: This function will be removed once the Pyth Core will be removed.
        bail!("Unimplemented")
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
