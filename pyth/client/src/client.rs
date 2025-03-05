use {
    grug::{Binary, Inner, JsonDeExt, Lengthy, NonEmpty, StdError},
    grug_app::Shared,
    indexer_disk_saver::{error::Error, persistence::DiskPersistence},
    pyth_types::LatestVaaResponse,
    reqwest::Client,
    reqwest_eventsource::{retry::ExponentialBackoff, Event, EventSource},
    sha2::{Digest, Sha256},
    std::{
        collections::HashMap,
        env,
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::{self, sleep},
        time::Duration,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
    tracing::{error, info},
};

pub struct PythClient {
    base_url: String,
    keep_running: Arc<AtomicBool>,
    middleware: Option<PythMockCache>,
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
    pub fn with_middleware(mut self) -> Self {
        self.middleware = Some(PythMockCache::new());
        self
    }

    /// Start a SSE connection to the Pyth network.
    /// If shared_vaas is provided, it will update the shared value with the latest VAA.
    /// Otherwise, it will create a new shared value and return it.
    pub fn run_streaming<I>(&mut self, ids: NonEmpty<I>) -> Shared<Vec<Binary>>
    where
        I: IntoIterator + Lengthy + Send + 'static,
        I::Item: ToString,
    {
        // Close the previous connection if it exists since the Arc
        // to shut down the thread will be replaced.
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
        if let Some(_) = &mut self.middleware {
            let mut middleware = PythMockCache::new();
            thread::spawn(move || {
                middleware.run_streaming(ids, shared_clone, keep_running_clone);
            });

            return shared;
        }

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
    pub fn get_latest_vaas<I>(&mut self, ids: I) -> reqwest::Result<Vec<Binary>>
    where
        I: IntoIterator + Clone,
        I::Item: ToString,
    {
        // If there is the middleware, try to get the data from it.
        if let Some(middleware) = &mut self.middleware {
            // This code should be reached only in tests.
            // Unwrap in order to fail if the file is not found.
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

    fn create_request_params<I>(ids: I) -> Vec<(&'static str, String)>
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let mut params = ids
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
        let params = PythClient::create_request_params(ids.into_inner());

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
                    Ok(Event::Open) => info!("Pyth SSE connection open"),
                    Ok(Event::Message(message)) => {
                        // Deserialize the message.
                        match message.data.deserialize_json::<LatestVaaResponse>() {
                            Ok(vaas) => {
                                // Check if the thread should keep running.
                                if !keep_running.load(Ordering::Relaxed) {
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

pub struct PythMockCache {
    stored_vaas: HashMap<String, std::vec::IntoIter<Vec<Binary>>>,
}

impl PythMockCache {
    pub fn new() -> Self {
        Self {
            stored_vaas: HashMap::new(),
        }
    }

    /// Inner function to run the SSE connection.
    fn run_streaming<I>(
        &mut self,
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
            .map(|id| id.to_string())
            .collect::<Vec<_>>();

        loop {
            // Check if the thread should keep running.
            if !keep_running.load(Ordering::Relaxed) {
                return;
            }

            // Retrieve the vaas
            let vaas = self.get_latest_vaas(ids.clone()).unwrap();

            // Update the shared value.
            shared.write_with(|mut shared_vaas| {
                *shared_vaas = vaas;
            });

            // Sleep for 0,5 seconds.
            sleep(Duration::from_millis(500));
        }
    }

    /// Get the latest VAAs from cached data.
    pub fn get_latest_vaas<I>(&mut self, ids: I) -> Result<Vec<Binary>, Error>
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let mut return_vaas = vec![];

        // For each id, try to get the vaas.
        for id in ids {
            let filename = self.create_file_name(vec![id]);

            // If the file is not in memory, try to read from disk.
            if !self.stored_vaas.contains_key(&filename) {
                let cache_file = DiskPersistence::new(filename.clone().into(), true);
                if cache_file.file_path.exists() {
                    let loaded_vaas = cache_file.load::<Vec<Vec<Binary>>>().unwrap();
                    self.stored_vaas
                        .insert(filename.clone(), loaded_vaas.into_iter());
                }
            }

            // Check if the vaas are stored in memory.
            if let Some(vaas_iter) = self.stored_vaas.get_mut(&filename) {
                if let Some(vaas) = vaas_iter.next() {
                    return_vaas.extend(vaas);
                }
            } else {
                return Err(StdError::DataNotFound {
                    ty: "cache",
                    key: filename,
                }
                .into());
            }
        }

        Ok(return_vaas)
    }

    /// Cache data.
    pub fn store_data<I>(&self, ids: I, data: Vec<Vec<Binary>>) -> Result<(), Error>
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let filename = self.create_file_name(ids);

        let cache_file = DiskPersistence::new(filename.clone().into(), true);
        cache_file.save(&data)?;

        Ok(())
    }

    fn create_file_name<I>(&self, ids: I) -> String
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut path = Path::new(&manifest_dir);

        // Risali alla root del workspace
        while path.parent().is_some() {
            path = path.parent().unwrap();
            let cargo_toml = path.join("Cargo.lock");
            if cargo_toml.exists() {
                break;
            }
        }

        // Sort the ids to have a unique file name.
        let mut ids = ids.into_iter().map(|id| id.to_string()).collect::<Vec<_>>();
        ids.sort();

        format!(
            "{}/pyth/client/testdata/{:x}",
            path.to_str().unwrap(),
            Sha256::digest(ids.join("__"))
        )
    }
}
