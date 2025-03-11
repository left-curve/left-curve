use {
    crate::middleware_cache::PythMiddlewareCache,
    async_stream::stream,
    grug::{Binary, Inner, JsonDeExt, Lengthy, NonEmpty},
    grug_app::Shared,
    pyth_types::LatestVaaResponse,
    reqwest::Client,
    reqwest_eventsource::{retry::ExponentialBackoff, Event, EventSource},
    std::{
        pin::Pin,
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
    // <U: IntoUrl>
    pub base_url: String, // U,
    keep_running: Arc<AtomicBool>,
    // I think since this is only for `PythMiddlewareCache`, it should be named
    // `middleware_cache` instead of `middleware`, or even better just have
    // `cache_enabled: true`.
    // Or we should have a MiddlewareTrait instead of `PythMiddlewareCache`.
    middleware: Option<PythMiddlewareCache>,
}

impl PythClient {
    // Why not use `IntoUrl` ?
    pub fn new<S: ToString>(base_url: S) -> Self {
        Self {
            base_url: base_url.to_string(),
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
    #[deprecated]
    pub fn run_streaming<I>(&mut self, ids: NonEmpty<I>) -> Shared<Vec<Binary>>
    where
        I: IntoIterator + Lengthy + Send + Clone + 'static,
        I::Item: ToString,
    {
        // Close the previous connection if it exists since the Arc
        // to shut down the thread will be replaced.
        // NOTE: this will not stop the thread immediately.
        self.close();

        let base_url = self.base_url.clone();

        // Create the shared vector to write/read the vaas.
        let shared = Shared::new(vec![]);
        let shared_clone = shared.clone();

        // Create a new atomic bool. Don't reuse the old one since there is no
        // guarantee that the old thread has already stopped.
        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running_clone = self.keep_running.clone();

        // If the middleware, run the middleware thread.
        if let Some(middleware) = &self.middleware {
            middleware.run_streaming(ids, base_url, shared_clone, keep_running_clone);
        } else {
            thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let mut stream = PythClient::stream(&base_url, ids).await.unwrap();

                    loop {
                        tokio::select! {
                            _ = tokio::time::sleep(tokio::time::Duration::from_millis(1000)) => {
                                if !keep_running_clone.load(Ordering::Relaxed) {
                                    return;
                                }
                            }

                            data = stream.next() => {
                                // to avoid waiting for the next second tick
                                if !keep_running_clone.load(Ordering::Relaxed) {
                                    return;
                                }

                                if let Some(data) = data {
                                    shared_clone.write_with(|mut shared_vaas| *shared_vaas = data);
                                }
                            }

                        }
                    }
                });
            });
        }

        shared
    }

    /// Stop the streaming thread.
    pub fn close(&mut self) {
        // This doesn't stop the streaming thread immediately, but it will stop
        // after the next message is received *and* the message is properly
        // deserialized as a `LatestVaaResponse`.

        self.keep_running.store(false, Ordering::SeqCst);
    }

    /// Get the latest VAA from the Pyth network. Only used for testing.
    pub fn get_latest_vaas<I>(&mut self, ids: NonEmpty<I>) -> reqwest::Result<Vec<Binary>>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        // If there is the middleware, try to get the data from it.
        if let Some(middleware) = &mut self.middleware {
            // This code should be reached only in tests.
            // Unwrap in order to fail if the cached data are not found.
            return Ok(middleware
                .get_latest_vaas(ids.clone(), &self.base_url)
                .unwrap());
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

    pub async fn stream<I>(
        base_url: &str,
        ids: NonEmpty<I>,
    ) -> Result<
        Pin<Box<dyn tokio_stream::Stream<Item = Vec<Binary>> + Send>>,
        reqwest_eventsource::CannotCloneRequestError,
    >
    where
        I: IntoIterator + Lengthy,
        I::Item: ToString,
    {
        let params = PythClient::create_request_params(ids);
        let builder = Client::new()
            .get(format!("{}/v2/updates/price/stream", base_url))
            .query(&params);

        // Connect to EventSource.
        let mut es = EventSource::new(builder)?;

        // Set the exponential backoff for reconnect.
        es.set_retry_policy(Box::new(ExponentialBackoff::new(
            Duration::from_secs(1),
            1.5,
            Some(Duration::from_secs(30)),
            None,
        )));

        let stream = stream! {
            loop {
                while let Some(event) = es.next().await {
                    match event {
                        Ok(Event::Open) => debug!("Pyth SSE connection open"),
                        Ok(Event::Message(message)) => {
                            match message.data.deserialize_json::<LatestVaaResponse>() {
                                Ok(vaas) => {
                                    yield vaas.binary.data;
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
        };

        Ok(Box::pin(stream))
    }
}
