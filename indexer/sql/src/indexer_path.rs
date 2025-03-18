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
