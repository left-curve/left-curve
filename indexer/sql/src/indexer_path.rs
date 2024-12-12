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
        Self::TempDir(Arc::new(tempfile::tempdir().expect("can't get a tempdir")))
    }
}

impl IndexerPath {
    /// Will be used when storing blocks long term to allow reindexing
    pub fn block_path(&self) -> PathBuf {
        match self {
            IndexerPath::TempDir(tmpdir) => tmpdir.path().join("blocks"),
            IndexerPath::Dir(dir) => dir.join("blocks"),
        }
    }

    /// Create all the needed subdirectories to avoid error when saving files into those
    pub fn create_dirs_if_needed(&self) -> error::Result<()> {
        let block_path = self.block_path();
        if !block_path.exists() {
            std::fs::create_dir_all(&block_path)?;
        }
        Ok(())
    }
}
