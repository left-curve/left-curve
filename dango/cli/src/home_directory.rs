use std::path::PathBuf;

/// Grug needs to store some data in different folders
pub struct HomeDirectory {
    home: PathBuf,
}

impl HomeDirectory {
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }

    /// Used for the RocksDB database.
    pub fn data_dir(&self) -> PathBuf {
        self.home.join("data")
    }

    /// Used for keystores.
    pub fn keys_dir(&self) -> PathBuf {
        self.home.join("keys")
    }

    /// Used for the indexer, used to store blocks before they're saved to the DB.
    pub fn indexer_dir(&self) -> PathBuf {
        self.home.join("indexer")
    }

    pub fn config_file(&self) -> PathBuf {
        self.home.join("app.toml")
    }
}
