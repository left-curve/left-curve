use {
    crate::error,
    std::{path::PathBuf, sync::Arc},
};

#[derive(Debug, Clone)]
pub enum IndexerPath {
    /// Tempdir is used for test, and will be automatically deleted once out of scope
    TempDir(Arc<tempfile::TempDir>),
    /// Directory to store the next block to be indexed
    Dir(PathBuf),
}

impl Default for IndexerPath {
    fn default() -> Self {
        Self::new_with_tempdir()
    }
}

impl IndexerPath {
    pub fn new_with_tempdir() -> Self {
        match tempfile::tempdir() {
            Ok(temp_dir) => Self::TempDir(Arc::new(temp_dir)),
            Err(_) => {
                let pid = std::process::id();
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                let fallback =
                    std::env::temp_dir().join(format!("indexer-cache-{pid}-{timestamp}"));
                Self::Dir(fallback)
            },
        }
    }

    pub fn new_with_dir(directory: PathBuf) -> Self {
        Self::Dir(directory)
    }

    /// Will be used when storing blocks long term to allow reindexing
    pub fn blocks_path(&self) -> PathBuf {
        match self {
            IndexerPath::TempDir(tmpdir) => tmpdir.path().join("blocks"),
            IndexerPath::Dir(dir) => dir.join("blocks"),
        }
    }

    /// Will be used when storing blocks long term to allow reindexing
    pub fn block_path(&self, block_height: u64) -> PathBuf {
        let mut elements = Vec::new();

        elements.push(self.blocks_path());

        elements.append(
            &mut block_height
                .to_string()
                .chars()
                .rev()
                .take(3)
                .map(|x| x.to_string().into())
                .collect::<Vec<PathBuf>>(),
        );

        elements.push(block_height.to_string().into());

        elements.into_iter().collect()
    }

    /// Create all the needed subdirectories to avoid error when saving files into those
    pub fn create_dirs_if_needed(&self) -> error::Result<()> {
        let block_path = self.blocks_path();
        if !block_path.exists() {
            std::fs::create_dir_all(&block_path)?;
        }
        Ok(())
    }
}
