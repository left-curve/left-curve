use {
    crate::{PythClient, PythClientTrait},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Inner, Lengthy, NonEmpty},
    indexer_disk_saver::persistence::DiskPersistence,
    pyth_types::{LeEcdsaMessage, PriceUpdate, PythId, PythLazerSubscriptionDetails},
    reqwest::IntoUrl,
    std::{
        collections::HashMap,
        env,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
    tracing::warn,
};

/// Define the number of samples for each PythId to store in file.
pub const PYTH_CACHE_SAMPLES: usize = 50;

#[derive(Debug, Clone)]
pub struct PythClientCache {
    client: PythClient,
    // Used to return newer vaas at each call.
    keep_running: Arc<AtomicBool>,
}

impl PythClientCache {
    pub fn new<V, U, T>(endpoints: NonEmpty<V>, access_token: T) -> Result<Self, anyhow::Error>
    where
        V: IntoIterator<Item = U> + Lengthy,
        U: IntoUrl,
        T: ToString,
    {
        let client = PythClient::new(endpoints, access_token)?;

        Ok(Self {
            client,
            keep_running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Load data from cache or retrieve it from the source.
    pub fn load_or_retrieve_data<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> HashMap<PathBuf, Vec<NonEmpty<Vec<LeEcdsaMessage>>>>
    where
        I: IntoIterator<Item = PythLazerSubscriptionDetails> + Lengthy + Clone,
    {
        let mut stored_data = HashMap::new();

        // Load data for each id.
        for subscription_details in ids.into_inner() {
            let filename = Self::cache_filename(&subscription_details.id);

            // If the file is not in memory, try to read from disk.
            stored_data.entry(filename.clone()).or_insert_with(|| {
                let mut cache_file = DiskPersistence::new(filename, true);

                if cache_file.exists() {
                    match cache_file.load() {
                        Ok(values) => return values,
                        Err(err) => {
                            warn!(error = %err, "failed to load cached Pyth samples, reloading from stream");
                        },
                    }
                }

                let rt = match Runtime::new() {
                    Ok(rt) => rt,
                    Err(err) => {
                        warn!(error = %err, "failed to create Tokio runtime for Pyth cache prefill");
                        return vec![];
                    },
                };
                let values = rt.block_on(async {
                    let mut stream = match self
                        .client
                        .stream(NonEmpty::new_unchecked(vec![subscription_details]))
                        .await
                    {
                        Ok(stream) => stream,
                        Err(err) => {
                            warn!(error = %err, "failed to open Pyth stream for cache prefill");
                            return vec![];
                        },
                    };

                    // Retrieve CACHE_SAMPLES values to be able to return newer values each time.
                    let mut values = vec![];
                    while values.len() < PYTH_CACHE_SAMPLES {
                        if let Some(price_update) = stream.next().await {
                            values.push(price_update);
                        }
                    }

                    values
                });

                // Store the data in the cache.
                if let Err(err) = cache_file.save(&values) {
                    warn!(error = %err, "failed to save prefetched Pyth cache data");
                }
                values
            });
        }

        stored_data
    }

    pub fn cache_filename(id: &PythId) -> PathBuf {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));

        let start_path = Path::new(&manifest_dir);

        let workspace_root = start_path
            .ancestors()
            .find(|p| p.join("Cargo.lock").exists())
            .unwrap_or(start_path);

        workspace_root
            .join("pyth/client/testdata")
            .join(id.to_string())
    }
}

#[async_trait]
impl PythClientTrait for PythClientCache {
    type Error = anyhow::Error;

    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = PriceUpdate> + Send>>, Self::Error>
    where
        I: IntoIterator<Item = PythLazerSubscriptionDetails> + Lengthy + Send + Clone,
    {
        self.close();
        self.keep_running = Arc::new(AtomicBool::new(true));

        let keep_running = self.keep_running.clone();
        let mut stored_data = self.load_or_retrieve_data(ids);
        let mut index = 0;

        let stream = stream! {
            loop {
                if !keep_running.load(Ordering::Acquire) {
                    return;
                }

                let data = stored_data
                    .iter_mut()
                    .filter_map(|(_, v)| v.get(index).cloned())
                    .flat_map(|v| v.into_inner())
                    .collect::<Vec<_>>();

                index += 1;

                if data.is_empty() {
                    warn!("No new VAA data available, waiting for next update");
                } else {
                    yield NonEmpty::new_unchecked(data);
                }

                sleep(Duration::from_millis(400));
            }
        };

        Ok(Box::pin(stream))
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
