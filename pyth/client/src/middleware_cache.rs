use {
    grug::{Binary, Inner, Lengthy, NonEmpty, StdError},
    grug_app::Shared,
    indexer_disk_saver::{error::Error, persistence::DiskPersistence},
    std::{
        collections::HashMap,
        env,
        path::Path,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread::sleep,
        time::Duration,
    },
};

pub struct PythMiddlewareCache {
    stored_vaas: HashMap<String, std::vec::IntoIter<Vec<Binary>>>,
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

    /// Inner function to run the SSE connection.
    pub fn run_streaming<I>(
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
    pub fn get_latest_vaas<I>(&mut self, ids: NonEmpty<I>) -> Result<Vec<Binary>, Error>
    where
        I: IntoIterator + Lengthy,
        I::Item: ToString,
    {
        let mut return_vaas = vec![];

        // For each id, try to get the vaas.
        for id in ids.into_inner() {
            let filename = self.create_file_name(id);

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
    pub fn store_data<I>(&self, id: I, data: Vec<Vec<Binary>>) -> Result<(), Error>
    where
        I: ToString,
    {
        let filename = self.create_file_name(id);

        let cache_file = DiskPersistence::new(filename.clone().into(), true);
        cache_file.save(&data)?;

        Ok(())
    }

    fn create_file_name<I>(&self, id: I) -> String
    where
        I: ToString,
    {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut path = Path::new(&manifest_dir);

        // Find the workspace path.
        while path.parent().is_some() {
            path = path.parent().unwrap();
            let cargo_toml = path.join("Cargo.lock");
            if cargo_toml.exists() {
                break;
            }
        }

        format!(
            "{}/pyth/client/testdata/{}",
            path.to_str().unwrap(),
            id.to_string()
        )
    }
}
