use {
    crate::middleware_cache::PythMiddlewareCache,
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
    tracing::{debug, error},
};

/// PythClient is a client to interact with the Pyth network.
pub struct PythClient {
    base_url: String,
    keep_running: Arc<AtomicBool>,
    middleware: Option<PythMiddlewareCache>,
}

impl PythClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            keep_running: Arc::new(AtomicBool::new(false)),
            middleware: None,
        }
    }

    /// Use the middleware to be able to use cached data.
    pub fn with_middleware_cache(mut self) -> Self {
        self.middleware = Some(PythMiddlewareCache::new());
        self
    }

    /// Start a SSE connection to the Pyth network and close the previous one if it exists.
    /// Return a shared vector to read the vaas.
    /// If the middleware is used, the function will run the middleware thread.
    pub fn run_streaming<I>(&mut self, ids: NonEmpty<I>) -> Shared<Vec<Binary>>
    where
        I: IntoIterator + Lengthy + Send + Clone + 'static,
        I::Item: ToString,
    {
        // Close the previous connection if it exists since the Arc
        // to shut down the thread will be replaced.
        self.close();

        // Create the shared vector to write/read the vaas.
        let shared = Shared::new(vec![]);
        let shared_clone = shared.clone();

        // Create a new atomic bool. Don't reuse the old one since there is no
        // guarantee that the old thread has already stopped.
        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running_clone = self.keep_running.clone();

        // If the middleware, run the middleware thread.
        if self.middleware.is_some() {
            thread::spawn(move || {
                let mut pyth_mock = PythMiddlewareCache::new();
                pyth_mock.run_streaming(ids, shared_clone, keep_running_clone);
            });
        } else {
            let base_url = self.base_url.clone();

            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    PythClient::run_streaming_inner(
                        base_url,
                        ids,
                        shared_clone,
                        keep_running_clone,
                    )
                    .await;
                });
            });
        }

        shared
    }

    /// Stop the streaming thread.
    pub fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }

    /// Get the latest VAA from the Pyth network.
    pub fn get_latest_vaas<I>(&mut self, ids: NonEmpty<I>) -> reqwest::Result<Vec<Binary>>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        // If there is the middleware, try to get the data from it.
        if let Some(middleware) = &mut self.middleware {
            // This code should be reached only in tests.
            // Unwrap in order to fail if the cached data are not found.
            return Ok(middleware.get_latest_vaas(ids.clone()).unwrap());
        }

        Ok(reqwest::blocking::Client::new()
            .get(format!("{}/v2/updates/price/latest", self.base_url))
            .query(&PythClient::create_request_params(ids.clone()))
            .send()?
            .error_for_status()?
            .json::<LatestVaaResponse>()?
            .binary
            .data)
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
        let params = PythClient::create_request_params(ids);

        loop {
            let builder = Client::new()
                .get(format!("{}/v2/updates/price/stream", base_url))
                .query(&params);

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
                    Ok(Event::Open) => debug!("Pyth SSE connection open"),
                    Ok(Event::Message(message)) => {
                        // Deserialize the message.
                        match message.data.deserialize_json::<LatestVaaResponse>() {
                            Ok(vaas) => {
                                // Check if the thread should keep running.
                                if !keep_running.load(Ordering::Relaxed) {
                                    debug!("Pyth SSE connection open");
                                    return;
                                }

                                // Update the shared value.
                                shared
                                    .write_with(|mut shared_vaas| *shared_vaas = vaas.binary.data);
                            },
                            Err(err) => {
                                error!(
                                    err = err.to_string(),
                                    "Failed to deserialize Pyth event into LatestVaaResponse"
                                );
                            },
                        }
                    },
                    Err(err) => {
                        error!(
                            err = err.to_string(),
                            "Error while receiving the events from Pyth"
                        );
                        es.close();
                    },
                }
            }
        }
    }
}
