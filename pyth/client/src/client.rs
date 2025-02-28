use {
    grug::{Binary, Inner, JsonDeExt, Lengthy, NonEmpty},
    grug_app::Shared,
    pyth_types::LatestVaaResponse,
    reqwest::Client,
    reqwest_eventsource::{retry::ExponentialBackoff, Event, EventSource},
    std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::{self},
        time::Duration,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
    tracing::{error, info},
};

pub struct PythClient {
    base_url: String,
    keep_running: Arc<AtomicBool>,
}

impl PythClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            keep_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start a SSE connection to the Pyth network.
    /// If shared_vaas is provided, it will update the shared value with the latest VAA.
    /// Otherwise, it will create a new shared value and return it.
    pub fn run_streaming<I>(
        &mut self,
        ids: NonEmpty<I>,
        shared_vaas: Option<Shared<Vec<Binary>>>,
    ) -> Shared<Vec<Binary>>
    where
        I: IntoIterator + Lengthy + Send + 'static,
        I::Item: ToString,
    {
        // Close the previous connection if it exists since the Arc
        // to shut down the thread will be replaced.
        self.close();

        let base_url = self.base_url.clone();

        let shared = shared_vaas.unwrap_or_default();
        let shared_clone = shared.clone();

        // Create a new atomic bool. Don't reuse the old one since there is no
        // guarantee that the old thread has already stopped.
        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running_clone = self.keep_running.clone();

        thread::spawn(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                PythClient::run_streaming_inner(base_url, ids, shared_clone, keep_running_clone)
                    .await;
                info!("Pyth SSE connection closed");
            });
        });

        shared
    }

    /// Close the client and stop the streaming thread.
    pub fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }

    /// Get the latest VAA from the Pyth network.
    pub fn get_latest_vaas<I>(&self, ids: I) -> reqwest::Result<Vec<Binary>>
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let ids = ids
            .into_iter()
            .map(|id| ("ids[]", id.to_string()))
            .collect::<Vec<_>>();

        Ok(reqwest::blocking::Client::new()
            .get(format!("{}/api/latest_vaas", self.base_url))
            .query(&ids)
            .send()?
            .error_for_status()?
            .json::<LatestVaaResponse>()?
            .binary
            .data)
    }

    /// Inner function to run the SSE connection.
    async fn run_streaming_inner<I>(
        base_url: String,
        ids: NonEmpty<I>,
        shared: Shared<Vec<Binary>>,
        keep_running: Arc<AtomicBool>,
    ) where
        I: IntoIterator + Lengthy,
        I::Item: ToString,
    {
        let ids = ids
            .into_inner()
            .into_iter()
            .map(|id| ("ids[]", id.to_string()))
            .collect::<Vec<_>>();

        loop {
            let builder = Client::new()
                .get(format!("{}/v2/updates/price/stream", base_url))
                .query(&ids)
                .query(&[("parsed", "false")])
                .query(&[("encoding", "base64")])
                // If, for some reason, the price id is invalid, ignore it
                // instead of making the request fail.
                // TODO: remove? The request still fail with random id. To be ignored,
                // the id must respect the correct id format (len, string in hex).
                .query(&[("ignore_invalid_price_ids", "true")]);

            // Connect to EventSource.
            // This method will return Err only if the RequestBuilder cannot be cloned.
            // This only happens if the request body is a stream (not this case).
            let mut es = EventSource::new(builder).unwrap();

            // Set the exponential backoff for reconnect.
            es.set_retry_policy(Box::new(ExponentialBackoff::new(
                Duration::from_secs(1),
                1.5,
                Some(Duration::from_secs(30)),
                None,
            )));

            // Waiting for next message and send through channel.
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => info!("Pyth SSE connection open"),
                    Ok(Event::Message(message)) => {
                        if let Ok(vaas) = message.data.deserialize_json::<LatestVaaResponse>() {
                            // Check if the thread should keep running.
                            if !keep_running.load(Ordering::Relaxed) {
                                return;
                            }

                            shared.write_with(|mut shared_vaas| *shared_vaas = vaas.binary.data);
                        } else {
                            error!("Failed to deserialize the message: {:?}", message);
                        }
                    },
                    Err(err) => {
                        error!("Error: {}", err);
                        es.close();
                    },
                }
            }
        }
    }
}
