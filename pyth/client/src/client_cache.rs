use {
    crate::{client::PythClientTrait, error, PythClient},
    async_stream::stream,
    async_trait::async_trait,
    grug::{Binary, Inner, Lengthy, NonEmpty},
    indexer_disk_saver::{error::Error, persistence::DiskPersistence},
    reqwest::{IntoUrl, Url},
    std::{
        collections::HashMap,
        env,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        thread::sleep,
        time::Duration,
    },
    tokio::runtime::Runtime,
    tokio_stream::StreamExt,
};

#[derive(Debug, Clone)]
pub struct PythClientCache {
    base_url: Url,
    // Use mutex to have inner mutability since PythClientTrait
    // requires immutable references in some functions.
    memory_vaas: Arc<Mutex<HashMap<PathBuf, std::vec::IntoIter<Vec<Binary>>>>>,
}

impl PythClientCache {
    pub fn new<U: IntoUrl>(base_url: U) -> Result<Self, error::Error> {
        Ok(Self {
            base_url: base_url.into_url()?,
            memory_vaas: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Load data from cache or retrieve it from the source.
    pub fn load_or_retrieve_data<I>(
        base_url: Url,
        ids: NonEmpty<I>,
    ) -> HashMap<PathBuf, std::vec::IntoIter<Vec<Binary>>>
    where
        I: IntoIterator + Lengthy + Clone,
        I::Item: ToString,
    {
        let mut stored_vaas = HashMap::new();

        // Load data for each id.
        for id in ids.into_inner() {
            let element = id.to_string();
            let filename = Self::cache_filename(&element);

            #[allow(clippy::map_entry)]
            // If the file is not in memory, try to read from disk.
            if !stored_vaas.contains_key(&filename) {
                let cache_file = DiskPersistence::new(filename.clone(), true);

                // If the file does not exists, retrieve the data from the source.
                if !cache_file.exists() {
                    let rt = Runtime::new().unwrap();
                    let values = rt.block_on(async {
                        let client = PythClient::new(base_url.clone()).unwrap();

                        let mut stream = client
                            .stream(NonEmpty::new(vec![id.to_string()]).unwrap())
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
                    Self::store_data(element, values).unwrap();
                }

                // Load the data from disk.
                let loaded_vaas = cache_file.load::<Vec<Vec<Binary>>>().unwrap();
                stored_vaas.insert(filename, loaded_vaas.into_iter());
            }
        }

        stored_vaas
    }

    /// Cache data.
    pub fn store_data<I>(id: I, data: Vec<Vec<Binary>>) -> Result<(), Error>
    where
        I: AsRef<Path>,
    {
        let filename = Self::cache_filename(&id);

        let cache_file = DiskPersistence::new(filename, true);
        cache_file.save(&data)?;

        Ok(())
    }

    // Load the data and save it in memory, so the lastest_vaas function
    // can return new data at each call.
    fn load_data_in_memory<I>(&self, ids: NonEmpty<I>)
    where
        I: IntoIterator + Lengthy + Clone,
        I::Item: ToString,
    {
        let mut vaas_in_memory = self.memory_vaas.lock().unwrap();

        // Check which files are not in memory.
        let mut ids_to_retrieve = vec![];
        for id in ids.into_inner() {
            let element = id.to_string();
            let filename = Self::cache_filename(&element);

            if vaas_in_memory.contains_key(&filename) {
                continue;
            }

            ids_to_retrieve.push(element);
        }

        // Retrieve the missing ids and store in memory.
        if !ids_to_retrieve.is_empty() {
            let base_url = self.base_url.clone();
            let stored_vaas =
                Self::load_or_retrieve_data(base_url, NonEmpty::new(ids_to_retrieve).unwrap());

            for (path, iterator) in stored_vaas {
                vaas_in_memory.insert(path, iterator);
            }
        }
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
        &self,
        ids: NonEmpty<I>,
    ) -> Result<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Vec<Binary>> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString,
    {
        let mut stored_vaas = Self::load_or_retrieve_data(self.base_url.clone(), ids);

        let stream = stream! {

            loop {
                let vaas = stored_vaas
                .iter_mut()
                .filter_map(|(_, v)| v.next())
                .flatten()
                .collect::<Vec<_>>();

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
        // Load the data in memory.
        self.load_data_in_memory(ids.clone());
        let mut vaas_in_memory = self.memory_vaas.lock().unwrap();

        let mut return_vaas = vec![];

        // For each id, try to get the vaas.
        for id in ids.into_inner() {
            let element = id.to_string();
            let filename = Self::cache_filename(&element);

            // Check if the vaas are stored in memory.
            if let Some(vaas_iter) = vaas_in_memory.get_mut(&filename) {
                if let Some(vaas) = vaas_iter.next() {
                    return_vaas.extend(vaas);
                }
            } else {
                return Err(error::Error::DataNotFound {
                    ty: "cache",
                    key: filename.to_string_lossy().to_string(),
                });
            }
        }

        Ok(return_vaas)
    }

    fn close(&mut self) {}
}
