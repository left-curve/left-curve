use {
    anyhow::bail,
    async_stream::stream,
    grug::{Inner, Lengthy, NonEmpty},
    pyth_client::PythClientTrait,
    pyth_lazer_client::{client::PythLazerClientBuilder, ws_connection::AnyResponse},
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
    },
    tracing::{error, info, warn},
    url::Url,
};

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

    /// Analyze the data received from the Pyth Lazer stream.
    fn analyze_data(
        data: AnyResponse,
        subscription_ids: &Vec<u64>,
        subscriptions_data: &mut HashMap<u64, pyth_types::LeEcdsaMessage>,
    ) {
        match data {
            AnyResponse::Binary(update) => {
                // Check the update is the current subscription ID.
                if !subscription_ids.contains(&update.subscription_id.0) {
                    warn!(
                        "Received update for a different subscription ID: {}. Expected: {:?}",
                        update.subscription_id.0, subscription_ids
                    );
                }

                let num_messages = update.messages.len();

                if num_messages == 0 {
                    warn!("Received empty update from Pyth Lazer stream");
                    return;
                }

                if num_messages > 1 {
                    error!(
                        "Received multiple messages in a single update, processing the first one"
                    );
                }

                let message = update.messages.first().unwrap().clone();

                match message {
                    Message::LeEcdsa(le_ecdsa_message) => {
                        // Since there are multiple subscriptions, we need to keep the last data for each subscription.
                        subscriptions_data
                            .insert(update.subscription_id.0, le_ecdsa_message.into());
                    },
                    _ => {
                        error!("Received non-ECDSA message: {:#?}", message);
                    },
                }
            },

            AnyResponse::Json(response) => match response {
                // TODO How to handle subscription errors?
                Response::Error(error_response) => {
                    error!("Failed to subscribe: {:#?}", error_response);
                },
                Response::Subscribed(subscription_response) => {
                    info!(
                        "Subscribed with ID: {}",
                        subscription_response.subscription_id.0
                    );
                },
                Response::SubscribedWithInvalidFeedIdsIgnored(subscription_response) => {
                    if subscription_response
                        .ignored_invalid_feed_ids
                        .unknown_ids
                        .is_empty()
                    {
                        info!(
                            "Subscribed to Pyth Lazer stream with subscription ID: {}",
                            subscription_response.subscription_id.0
                        );
                    } else {
                        warn!(
                            "Subscribed to Pyth Lazer stream with subscription ID: {} but some feed ids were ignored: {:#?}",
                            subscription_response.subscription_id.0,
                            subscription_response.ignored_invalid_feed_ids
                        );
                    }
                },
                Response::Unsubscribed(unsubscribed_response) => {
                    info!(
                        "Unsubscribed from Pyth Lazer stream with subscription ID: {}",
                        unsubscribed_response.subscription_id.0
                    );
                },
                Response::SubscriptionError(subscription_error_response) => {
                    error!(
                        "Failed to subscribe to Pyth Lazer stream: {:#?}",
                        subscription_error_response
                    );
                },
                Response::StreamUpdated(_) => {
                    error!("Received Lazer data in json format, only support Binary");
                },
            },
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

        // Build the new client and subscribe to the price feeds.
        let builder = PythLazerClientBuilder::new(self.access_token.clone())
            .with_endpoints(self.endpoints.clone());

        let mut client = builder.build()?;
        let mut receiver = client.start().await?;

        // Divide the ids depending on the channel.
        let mut ids_per_channel = HashMap::new();
        for value in ids.into_inner() {
            let channel = value.channel;
            ids_per_channel
                .entry(channel)
                .or_insert_with(Vec::new)
                .push(PriceFeedId(value.id));
        }

        // Create a subscription requests for each channel.
        let mut subscription_ids = Vec::with_capacity(ids_per_channel.len());

        let subscribe_requests = ids_per_channel
            .into_iter()
            .map(|(channel, ids)| {
                let params = Self::subscription_params(ids, channel)?;

                self.last_subscription_id += 1;
                subscription_ids.push(self.last_subscription_id);

                Ok(SubscribeRequest {
                    subscription_id: SubscriptionId(self.last_subscription_id),
                    params,
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;

        // Subscribe to the price feeds.
        // TODO: How to Handle the error here?
        for subscription in subscribe_requests {
            client
                .subscribe(subscription)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        // Since there are multiple subscriptions, we need to keep the last data for each subscriptions.
        let mut susbscriptions_data = HashMap::with_capacity(subscription_ids.len());

        let stream = stream! {
            loop {
                tokio::select! {
                    // The server is not sending any more data.
                    // Log the error and keep running, since the client will handle reconnection.
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {

                        // Check if the streaming has to be closed.
                        if !keep_running.load(Ordering::Relaxed) {
                            info!("Pyth Lazer connection closed");
                            break;
                        }

                        warn!("No new data received for a second");
                    },

                    // Read next data from stream.
                    data = receiver.recv() => {

                        // Check if the streaming has to be closed.
                        if !keep_running.load(Ordering::Acquire) {
                            info!("Pyth Lazer connection closed");
                            break;
                        }

                        // TODO: Handle the case when the connection is closed.
                        let Some(data) = data else {
                            error!("Pyth Lazer connection closed. Start reconnecting");
                            return;
                        };

                        // Analyze the data.
                        Self::analyze_data(data, &subscription_ids, &mut susbscriptions_data);

                        // Pull all data available from the receiver.
                        while !receiver.is_empty() {
                            let mut buffer = vec![];
                            receiver.recv_many(&mut buffer, 100).await;

                            // Analyze data.
                            for data in buffer {
                                Self::analyze_data(data, &subscription_ids, &mut susbscriptions_data);
                            }
                        }

                        // Yield the current data.
                        if !susbscriptions_data.is_empty(){
                            let send_data = susbscriptions_data.clone();
                            yield PriceUpdate::Lazer(NonEmpty::new_unchecked(send_data.into_values().collect::<Vec<_>>()));
                        }
                    }
                }
            }

            // If the code reaches here, it means the stream needs to be closed.
            for id in subscription_ids {
                match client.unsubscribe(SubscriptionId(id)).await {
                    Ok(_) => {info!("Unsubscribed stream id {} successfully", id);},
                    Err(e) => {error!("Failed to unsubscribe stream id {}: {:?}", id, e);},
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
