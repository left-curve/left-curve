use {
    crate::{PythClient, PythClientTrait, error},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Binary, Inner, Lengthy, NonEmpty},
    indexer_disk_saver::persistence::DiskPersistence,
    pyth_types::PriceUpdate,
    reqwest::{IntoUrl, Url},
    std::{
        collections::HashMap,
        env, panic,
        path::{Path, PathBuf},
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64, Ordering},
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
    base_url: Url,
    // Used to return newer vaas at each call.
    vaas_index: Arc<AtomicU64>,
    keep_running: Arc<AtomicBool>,
}

impl PythClientCache {
    pub fn new<U: IntoUrl>(base_url: U) -> Result<Self, error::Error> {
        Ok(Self {
            base_url: base_url.into_url()?,
            vaas_index: Arc::new(AtomicU64::new(0)),
            keep_running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Load data from cache or retrieve it from the source.
    pub fn load_or_retrieve_data<I>(
        base_url: &Url,
        ids: NonEmpty<I>,
    ) -> HashMap<PathBuf, Vec<NonEmpty<Vec<Binary>>>>
    where
        I: IntoIterator + Lengthy + Clone,
        I::Item: ToString,
    {
        let mut stored_vaas = HashMap::new();

        // Load data for each id.
        for id in ids.into_inner() {
            let filename = Self::cache_filename(&id.to_string());

            // If the file is not in memory, try to read from disk.
            stored_vaas.entry(filename.clone()).or_insert_with(|| {
                let mut cache_file = DiskPersistence::new(filename, true);

                if cache_file.exists() {
                    return cache_file.load::<Vec<NonEmpty<Vec<Binary>>>>().unwrap();
                }

                let rt = Runtime::new().unwrap();
                let values = rt.block_on(async {
                    let mut client = PythClient::new(base_url.clone()).unwrap();

                    let mut stream = client
                        .stream(NonEmpty::new(vec![id.to_string()]).unwrap())
                        .await
                        .unwrap();

                    // Retrieve CACHE_SAMPLES values to be able to return newer values each time.
                    let mut values = vec![];
                    while values.len() < PYTH_CACHE_SAMPLES {
                        if let Some(price_update) = stream.next().await {
                            if let PriceUpdate::Core(vaas) = price_update {
                                if vaas.is_empty() {
                                    warn!("Empty VAA received, skipping");
                                    continue;
                                }
                                values.push(vaas);
                            } else {
                                panic!("Received non-core PriceUpdate: {:?}", price_update);
                            }
                        }
                    }

                    values
                });

                // Store the data in the cache.
                cache_file.save(&values).unwrap();
                values
            });
        }

        stored_vaas
    }

    pub fn cache_filename<I>(id: &I) -> PathBuf
    where
        I: AsRef<Path>,
    {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

        let start_path = Path::new(&manifest_dir);

        let workspace_root = start_path
            .ancestors()
            .find(|p| p.join("Cargo.lock").exists())
            .expect("Workspace root not found");

        workspace_root.join("pyth/client/testdata").join(id)
    }
}

#[async_trait]
impl PythClientTrait for PythClientCache {
    type Error = error::Error;

    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = PriceUpdate> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString,
    {
        self.close();
        self.keep_running = Arc::new(AtomicBool::new(true));
        let keep_running = self.keep_running.clone();

        let mut stored_vaas = Self::load_or_retrieve_data(&self.base_url, ids);
        let mut index = 0;

        let stream = stream! {
            loop {
                if !keep_running.load(Ordering::Acquire) {
                    return;
                }

                let vaas = stored_vaas
                    .iter_mut()
                    .filter_map(|(_, v)| v.get(index).cloned()).map(|v| v.into_inner())
                    .flatten()
                    .collect::<Vec<_>>();

                index += 1;

                if vaas.is_empty() {
                    warn!("No new VAA data available, waiting for next update");
                }else{
                    yield PriceUpdate::Core(NonEmpty::new(vaas).unwrap());
                }

                sleep(Duration::from_millis(500));
            }
        };

        Ok(Box::pin(stream))
    }

    fn get_latest_price_update<I>(&self, ids: NonEmpty<I>) -> Result<PriceUpdate, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString,
    {
        // Load the data.
        let stored_vaas = Self::load_or_retrieve_data(&self.base_url, ids);
        let index = self.vaas_index.load(Ordering::Acquire) as usize;

        let result = stored_vaas
            .into_iter()
            .filter_map(|(_, v)| v.get(index).cloned())
            .map(|v| v.into_inner())
            .flatten()
            .collect::<Vec<_>>();

        self.vaas_index.fetch_add(1, Ordering::SeqCst);

        Ok(PriceUpdate::Core(NonEmpty::new(result)?))
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
