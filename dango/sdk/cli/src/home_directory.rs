use {
    anyhow::anyhow,
    home::home_dir,
    std::{
        ops::Deref,
        path::{Path, PathBuf},
    },
};

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

    fn config_dir(&self) -> PathBuf {
        self.home.join("config")
    }

    /// Return the path to the client configuration file.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir().join("client.toml")
    }

    /// Return the path to the directory that stores keys.
    pub fn keys_dir(&self) -> PathBuf {
        self.home.join("keys")
    }
}
