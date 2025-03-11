use {
    crate::PythClient,
    grug::{Binary, Inner, Lengthy, NonEmpty, StdError},
    grug_app::Shared,
    indexer_disk_saver::{error::Error, persistence::DiskPersistence},
    std::{
        collections::HashMap,
        env,
        path::{Path, PathBuf},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::{self, sleep},
        time::Duration,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
};

pub struct PythMiddlewareCache {
    stored_vaas: HashMap<PathBuf, std::vec::IntoIter<Vec<Binary>>>,
}

impl Default for PythMiddlewareCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PythMiddlewareCache {
    pub fn new() -> Self {
        Self {
            stored_vaas: HashMap::new(),
        }
    }

    /// Load data from cache (if not already loaded) or retrieve it from the source.
    pub fn load_or_retrieve_data<I, T>(&mut self, ids: NonEmpty<I>, base_url: T)
    where
        I: IntoIterator + Lengthy + Clone,
        I::Item: ToString,
        T: ToString,
    {
        // Load data for each id.
        for id in ids.into_inner() {
            let element = id.to_string();
            let filename = self.cache_filename(&element);

            #[allow(clippy::map_entry)]
            // If the file is not in memory, try to read from disk.
            if !self.stored_vaas.contains_key(&filename) {
                let cache_file = DiskPersistence::new(filename.clone(), true);

                // If the file does not exists, retrieve the data from the source.
                if !cache_file.exists() {
                    let rt = Runtime::new().unwrap();
                    let values = rt.block_on(async {
                        let mut stream = PythClient::stream(
                            &base_url.to_string(),
                            NonEmpty::new(vec![id.to_string()]).unwrap(),
                        )
                        .await
                        .unwrap();

                        let mut values = vec![];
                        while values.len() < 15 {
                            if let Some(vaas) = stream.next().await {
                                values.push(vaas);
                            }
                        }

                        values
                    });

                    // Store the data in the cache.
                    self.store_data(element, values).unwrap();
                }

                // Load the data from disk.
                let loaded_vaas = cache_file.load::<Vec<Vec<Binary>>>().unwrap();
                self.stored_vaas.insert(filename, loaded_vaas.into_iter());
            }
        }
    }

    /// Inner function to run the SSE connection.
    pub fn run_streaming<I>(
        &self,
        ids: NonEmpty<I>,
        base_url: String,
        shared: Shared<Vec<Binary>>,
        keep_running: Arc<AtomicBool>,
    ) where
        I: IntoIterator + Lengthy + Clone + Send + 'static,
        I::Item: ToString,
    {
        let mut pyth_mock = PythMiddlewareCache::new();
        pyth_mock.load_or_retrieve_data(ids.clone(), base_url);

        thread::spawn(move || {
            pyth_mock.run_streaming_inner(ids, shared, keep_running);
        });
    }

    /// Inner function to run the SSE connection.
    fn run_streaming_inner<I>(
        &mut self,
        ids: NonEmpty<I>,
        shared: Shared<Vec<Binary>>,
        keep_running: Arc<AtomicBool>,
    ) where
        I: IntoIterator + Lengthy + Clone,
        I::Item: ToString,
    {
        loop {
            // Check if the thread should keep running.
            if !keep_running.load(Ordering::Relaxed) {
                return;
            }

            // Retrieve the vaas (at this point, the vaas are already loaded in memory).
            let vaas: Vec<grug::EncodedBytes<Vec<u8>, grug::Base64Encoder>> =
                self.get_latest_vaas(ids.clone(), "").unwrap();

            // Update the shared value.

            shared.write_with(|mut shared_vaas| {
                *shared_vaas = vaas;
            });

            // Sleep for 0,5 seconds.
            sleep(Duration::from_millis(500));
        }
    }

    /// Get the latest VAAs from cached data.
    pub fn get_latest_vaas<I, T>(
        &mut self,
        ids: NonEmpty<I>,
        base_url: T,
    ) -> Result<Vec<Binary>, Error>
    where
        I: IntoIterator + Lengthy + Clone,
        I::Item: ToString,
        T: ToString,
    {
        self.load_or_retrieve_data(ids.clone(), base_url);

        let mut return_vaas = vec![];

        // For each id, try to get the vaas.
        for id in ids.into_inner() {
            let element = id.to_string();
            let filename = self.cache_filename(&element);

            // Check if the vaas are stored in memory.
            if let Some(vaas_iter) = self.stored_vaas.get_mut(&filename) {
                if let Some(vaas) = vaas_iter.next() {
                    return_vaas.extend(vaas);
                }
            } else {
                return Err(StdError::DataNotFound {
                    ty: "cache",
                    key: filename.to_string_lossy().to_string(),
                }
                .into());
            }
        }

        Ok(return_vaas)
    }

    /// Cache data.
    pub fn store_data<I>(&self, id: I, data: Vec<Vec<Binary>>) -> Result<(), Error>
    where
        I: AsRef<Path>,
    {
        let filename = self.cache_filename(&id);

        let cache_file = DiskPersistence::new(filename, true);
        cache_file.save(&data)?;

        Ok(())
    }

    fn cache_filename<I>(&self, id: &I) -> PathBuf
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
