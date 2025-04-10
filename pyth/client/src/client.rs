use {
    crate::{PythClientTrait, error},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Binary, Inner, JsonDeExt, Lengthy, NonEmpty},
    pyth_types::LatestVaaResponse,
    reqwest::{Client, IntoUrl, RequestBuilder, Url},
    reqwest_eventsource::{CannotCloneRequestError, Event, EventSource, retry::ExponentialBackoff},
    std::{
        cmp::min,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    },
    tokio::time::sleep,
    tokio_stream::StreamExt,
    tracing::{error, info, warn},
};

const MAX_DELAY: Duration = Duration::from_secs(10);

/// PythClient is a client to interact with the Pyth network.
#[derive(Debug, Clone)]
pub struct PythClient {
    pub base_url: Url,
    keep_running: Arc<AtomicBool>,
}

impl PythClient {
    pub fn new<U: IntoUrl>(base_url: U) -> Result<Self, error::Error> {
        Ok(Self {
            base_url: base_url.into_url()?,
            keep_running: Arc::new(AtomicBool::new(false)),
        })
    }

    fn create_request_params<I>(ids: NonEmpty<I>) -> Vec<(&'static str, String)>
    where
        I: IntoIterator + Lengthy,
        I::Item: ToString,
    {
        let mut params = ids
            .into_inner()
            .into_iter()
            .map(|id| ("ids[]", id.to_string()))
            .collect::<Vec<_>>();

        params.push(("parsed", "false".to_string()));
        params.push(("encoding", "base64".to_string()));
        // If, for some reason, the price id is invalid, ignore it
        // instead of making the request fail.
        // TODO: remove? The request still fail with random id. To be ignored,
        // the id must respect the correct id format (len, string in hex).
        params.push(("ignore_invalid_price_ids", "true".to_string()));

        params
    }

    async fn create_event_source(
        builder: &RequestBuilder,
        backoff_start_duration: Duration,
    ) -> EventSource {
        // Function to calculate the exponential backoff before try to reconnect.
        let fn_next_backoff_duration = |current_sleep: Duration| {
            if current_sleep > MAX_DELAY {
                MAX_DELAY
            } else {
                min(MAX_DELAY, current_sleep * 2)
            }
        };

        let mut current_sleep = backoff_start_duration;

        // Create the `EventSource`.
        loop {
            let mut es = match EventSource::new(builder.try_clone().unwrap()) {
                Ok(es) => es,
                Err(err) => {
                    error!(
                        err = err.to_string(),
                        "Failed to create EventSource. Reconnecting in {} seconds",
                        current_sleep.as_secs()
                    );

                    sleep(current_sleep).await;

                    current_sleep = fn_next_backoff_duration(current_sleep);

                    continue;
                },
            };

            // Set the exponential backoff for reconnect.
            es.set_retry_policy(Box::new(ExponentialBackoff::new(
                Duration::from_millis(500),
                1.5,
                Some(MAX_DELAY),
                None,
            )));

            // Check if the connection is open.
            match es.next().await {
                Some(event_result) => match event_result {
                    Ok(_) => {
                        info!("Pyth SSE connection open");

                        return es;
                    },
                    Err(err) => {
                        error!(
                            err = err.to_string(),
                            "Failed to connect. Reconnecting in {} seconds",
                            current_sleep.as_secs()
                        );
                    },
                },

                // This point should never be reached since the `EventSource` return None
                // only if the connection is closed with `close()`.
                // This check is made in case the `EventSource` logic changes in the future.
                None => {
                    error!(
                        "Received None. Reconnecting in {} seconds",
                        current_sleep.as_secs()
                    );
                },
            }

            // The connection is not open correctly: close it and wait before retrying.
            es.close();
            sleep(current_sleep).await;
            current_sleep = fn_next_backoff_duration(current_sleep);
        }
    }
}

#[async_trait]
impl PythClientTrait for PythClient {
    type Error = crate::error::Error;

    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = Vec<Binary>> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString,
    {
        // Close the previous connection.
        self.close();

        let params = PythClient::create_request_params(ids);
        let builder = Client::new()
            .get(self.base_url.join("v2/updates/price/stream")?)
            .query(&params);

        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running = self.keep_running.clone();

        // `EventSource::new()` return an Error only if the builder is not `Clone`-able.
        // Instead of returning a result inside of the stream, clone the builder
        // here to ensure it's cloneable.
        let _ = builder.try_clone().ok_or(CannotCloneRequestError)?;

        let stream = stream! {

            loop{
                // Create the `EventSource`.
                let mut es = Self::create_event_source(
                    &builder,
                    Duration::from_millis(500),
                )
                .await;

                loop {
                    tokio::select! {
                        // The server is not sending any more data. Drop connection and
                        // establish a new one.
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {
                            es.close();

                            // Check if the streaming has to be closed.
                            if !keep_running.load(Ordering::Relaxed) {
                                return;
                            }

                            warn!("No new data received. Start reconnecting");

                            es = Self::create_event_source(
                                &builder,
                                Duration::from_millis(100),
                            )
                            .await;
                        },

                        data = es.next() => {
                            if !keep_running.load(Ordering::Acquire) {
                                es.close();
                                return;
                            }

                            if let Some(event) = data {
                                match event {
                                    Ok(Event::Open) => {
                                        info!("Pyth SSE connection open");
                                    },
                                    Ok(Event::Message(message)) => {

                                        match message.data.deserialize_json::<LatestVaaResponse>() {
                                            Ok(vaas) => {
                                                yield vaas.binary.data;
                                            },
                                            Err(err) => {
                                                error!(
                                                    err = err.to_string(),
                                                    "Failed to deserialize Pyth event into `LatestVaaResponse`"
                                                );
                                            },
                                        }
                                    },
                                    Err(err) => {
                                        error!(
                                            err = err.to_string(),
                                            "Error while receiving the events from Pyth"
                                        );
                                    },
                                }
                            } else {
                                error!("Pyth SSE connection closed. Start reconnecting");
                                es.close();
                                break;
                            }
                        },
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn get_latest_vaas<I>(&self, ids: NonEmpty<I>) -> Result<Vec<Binary>, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        Ok(reqwest::blocking::Client::new()
            .get(self.base_url.join("v2/updates/price/latest")?)
            .query(&PythClient::create_request_params(ids.clone()))
            .send()?
            .error_for_status()?
            .json::<LatestVaaResponse>()?
            .binary
            .data)
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
