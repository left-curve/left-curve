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
        subscription::{SubscribeRequest, SubscriptionId},
    },
    pyth_types::PriceUpdate,
    std::{
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
    current_subscription_id: u64,
}

impl PythClientLazer {
    // TODO: shold we enforce endpoint to be NonEmpty or an Option in order to
    // be able to use the default one inside the sdk?
    pub fn new<T>(endpoints: Vec<Url>, access_token: T) -> Self
    where
        T: ToString,
    {
        PythClientLazer {
            endpoints,
            access_token: access_token.to_string(),
            keep_running: Arc::new(AtomicBool::new(false)),
            current_subscription_id: 0,
        }
    }
}

#[async_trait::async_trait]
impl PythClientTrait for PythClientLazer {
    type Error = anyhow::Error;

    // TODO: Change item type to u32 once Pyth Core is removed.
    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = PriceUpdate> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString,
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

        let price_feed_ids = ids
            .into_inner()
            .into_iter()
            .map(|value| value.to_string().parse::<u32>())
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|id| PriceFeedId(*id))
            .collect::<Vec<_>>();

        // Increment the subscription ID for each new subscription in order to filter
        // the updates correctly.
        self.current_subscription_id += 1;
        let current_subscription_id = self.current_subscription_id;

        let subscribe_request = SubscribeRequest {
            subscription_id: SubscriptionId(current_subscription_id),
            params: SubscriptionParams::new(SubscriptionParamsRepr {
                price_feed_ids,
                properties: vec![PriceFeedProperty::Price],
                formats: vec![Format::LeEcdsa],
                delivery_format: DeliveryFormat::Binary,
                json_binary_encoding: JsonBinaryEncoding::Base64,
                parsed: false,
                channel: Channel::RealTime,
                ignore_invalid_feed_ids: true,
            })
            .map_err(|e| anyhow::anyhow!(e))?,
        };

        client
            .subscribe(subscribe_request)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

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
                        let Some(mut data) = data else {
                            error!("Pyth Lazer connection closed. Start reconnecting");
                            return;
                        };

                        // The data is coming so fast that we have to drain the receiver
                        // and take the latest data.
                        loop {
                            match receiver.try_recv() {
                                Ok(newer_data) => {
                                    data = newer_data;
                                },
                                Err(_) => break,
                            }
                        }


                        match data {
                            AnyResponse::Binary(update) => {

                                // Check the update is the current subscription ID.
                                if update.subscription_id.0 != current_subscription_id {
                                    warn!("Received update for a different subscription ID: {}. Expected: {}", update.subscription_id.0, current_subscription_id);
                                }

                                let num_messages = update.messages.len();

                                if num_messages == 0 {
                                    warn!("Received empty update from Pyth Lazer stream");
                                    continue;
                                }

                                if num_messages > 1 {
                                    error!("Received multiple messages in a single update, processing the first one");
                                }

                                let message = update.messages.first().unwrap().clone();

                                match message {
                                    Message::LeEcdsa(le_ecdsa_message) => {

                                        yield PriceUpdate::Lazer(le_ecdsa_message.into());
                                    },
                                    _ => {
                                        error!("Received non-ECDSA message: {:#?}", message);
                                        continue;
                                    },
                                }

                            },
                            _ => {
                                error!("Received non-binary update: {:#?}", data);
                                continue;
                            },

                        }
                    }
                }
            }


            // If the code reaches here, it means the stream need to be closed.
            match client.unsubscribe(SubscriptionId(current_subscription_id)).await {
                Ok(_) => {info!("Unsubscribed from Pyth Lazer stream successfully");},
                Err(e) => {error!("Failed to unsubscribe from Pyth Lazer stream: {:?}", e);},
            };

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
