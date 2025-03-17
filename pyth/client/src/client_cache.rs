use {
    crate::{client::PythClientTrait, error, PythClient},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Binary, Inner, Lengthy, NonEmpty},
    indexer_disk_saver::persistence::DiskPersistence,
    reqwest::{IntoUrl, Url},
    std::{
        collections::HashMap,
        env,
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicBool, AtomicU64, Ordering},
            Arc,
        },
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
};

/// Define the number of samples for each PythId to store in file.
const CACHE_SAMPLES: usize = 15;

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
    ) -> HashMap<PathBuf, Vec<Vec<Binary>>>
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
                let cache_file = DiskPersistence::new(filename, true);

                if cache_file.exists() {
                    return cache_file.load::<Vec<Vec<Binary>>>().unwrap();
                }

                // If the file does not exists, retrieve the data from the source.
                let rt = Runtime::new().unwrap();
                let values = rt.block_on(async {
                    let mut client = PythClient::new(base_url.clone()).unwrap();

                    let mut stream = client
                        .stream(NonEmpty::new(vec![id.to_string()]).unwrap())
                        .await
                        .unwrap();

                    // Retrieve CACHE_SAMPLES values to be able to return newer values each time.
                    let mut values = vec![];
                    while values.len() < CACHE_SAMPLES {
                        if let Some(vaas) = stream.next().await {
                            values.push(vaas);
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

    fn cache_filename<I>(id: &I) -> PathBuf
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
    ) -> Result<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Vec<Binary>> + Send>>, Self::Error>
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
                .filter_map(|(_, v)| v.get(index).cloned())
                .flatten()
                .collect::<Vec<_>>();
                index += 1;

                yield vaas;
                sleep(Duration::from_millis(500));
            }
        };

        Ok(Box::pin(stream))
    }

    fn get_latest_vaas<I>(&self, ids: NonEmpty<I>) -> Result<Vec<Binary>, Self::Error>
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
            .flatten()
            .collect::<Vec<_>>();

        self.vaas_index.fetch_add(1, Ordering::SeqCst);

        Ok(result)
    }

    fn close(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}
