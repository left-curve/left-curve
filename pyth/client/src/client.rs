use {
    crate::{PythClientTrait, error},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Inner, JsonDeExt, Lengthy, NonEmpty},
    pyth_types::{LatestVaaResponse, PriceUpdate},
    reqwest::{Client, IntoUrl, Url},
    reqwest_eventsource::{Event, EventSource, retry::ExponentialBackoff},
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

const MAX_DELAY: Duration = Duration::from_secs(5);

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
        url: &Url,
        params: &Vec<(&str, String)>,
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
            // Create the request builder at new iterations to avoid the ‘try_clone()’ that
            // could lead to error.
            let builder = Client::new().get(url.clone()).query(params);

            let mut es = match EventSource::new(builder) {
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
                Duration::from_millis(100),
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
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = PriceUpdate> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString,
    {
        // Close the previous connection.
        self.close();

        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running = self.keep_running.clone();

        let url = self.base_url.join("v2/updates/price/stream")?;
        let params = PythClient::create_request_params(ids);

        let stream = stream! {
            loop {
                // Create the `EventSource`.
                let mut es = Self::create_event_source(&url, &params, Duration::from_millis(100)).await;

                loop {
                    tokio::select! {
                        // The server is not sending any more data.
                        // Drop connection and establish a new one.
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {
                            es.close();

                            // Check if the streaming has to be closed.
                            if !keep_running.load(Ordering::Relaxed) {
                                info!("Pyth SSE connection closed");
                                return;
                            }

                            warn!("No new data received. Start reconnecting");
                            break;
                        },

                        // Read next data from stream.
                        data = es.next() => {

                            // Check if the streaming has to be closed.
                            if !keep_running.load(Ordering::Acquire) {
                                es.close();
                                info!("Pyth SSE connection closed");
                                return;
                            }

                            // If connection is closed, try to reconnect.
                            let Some(event) = data else {
                                error!("Pyth SSE connection closed. Start reconnecting");
                                es.close();
                                break;
                            };

                            match event {
                                Ok(Event::Open) => {
                                    info!("Pyth SSE connection open");
                                },
                                Ok(Event::Message(message)) => {
                                    match message.data.deserialize_json::<LatestVaaResponse>() {
                                        Ok(vaas) => {
                                            if vaas.binary.data.is_empty() {
                                                continue;
                                            }

                                            yield PriceUpdate::Core(NonEmpty::new(vaas.binary.data).unwrap());
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
                        },
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn get_latest_vaas<I>(&self, ids: NonEmpty<I>) -> Result<PriceUpdate, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        let vaas = reqwest::blocking::Client::new()
            .get(self.base_url.join("v2/updates/price/latest")?)
            .query(&PythClient::create_request_params(ids.clone()))
            .send()?
            .error_for_status()?
            .json::<LatestVaaResponse>()?
            .binary
            .data;

        Ok(PriceUpdate::Core(NonEmpty::new(vaas)?))
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
