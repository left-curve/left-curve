use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use {anyhow::anyhow, home::home_dir};

// relative to user home directory (~)
const DEFAULT_APP_DIR: &str = ".dango";

pub struct HomeDirectory {
    pub home: PathBuf,
}

impl AsRef<Path> for HomeDirectory {
    fn as_ref(&self) -> &Path {
        &self.home
    }
}

impl Deref for HomeDirectory {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.home
    }
}

impl HomeDirectory {
    pub fn new_or_default(maybe_home: Option<PathBuf>) -> anyhow::Result<Self> {
        maybe_home
            .map_or_else(
                || {
                    let user_home = home_dir().ok_or(anyhow!("failed to find home directory"))?;
                    Ok(user_home.join(DEFAULT_APP_DIR))
                },
                Ok,
            )
            .map(|home| Self { home })
    }

    /// Return whether the home directory exists.
    pub fn exists(&self) -> bool {
        self.home.exists()
    }

    /// Return the path to the config directory.
    pub fn config_dir(&self) -> PathBuf {
        self.home.join("config")
    }

    /// Return the path to the configuration file.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("app.toml")
    }

    /// Return the path to Grug's RocksDB database directory.
    pub fn data_dir(&self) -> PathBuf {
        self.home.join("data")
    }

    /// Return the path to the directory used by the indexer to store blocks.
    pub fn indexer_dir(&self) -> PathBuf {
        self.home.join("indexer")
    }

    /// Return the path to the directory that stores keys.
    pub fn keys_dir(&self) -> PathBuf {
        self.home.join("keys")
    }
}
