use {
    rocksdb::{Options, DB},
    std::path::{Path, PathBuf},
    tempfile::TempDir,
};

/// Temporary database path which calls DB::destroy when DBPath is dropped.
/// Copyed from rust-rocksdb:
/// <https://github.com/rust-rocksdb/rust-rocksdb/blob/v0.21.0/tests/util/mod.rs#L8>
pub struct TempDataDir {
    #[allow(dead_code)]
    dir: TempDir, // keep the value alive so that the directory isn't deleted prematurely
    path: PathBuf,
}

impl TempDataDir {
    /// Produces a fresh (non-existent) temporary path which will be
    /// DB::destroy'ed automatically.
    pub fn new(prefix: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .unwrap_or_else(|err| {
                panic!("failed to create temporary directory for DB: {err}");
            });
        let path = dir.path().join("db");
        Self { dir, path }
    }
}

impl Drop for TempDataDir {
    fn drop(&mut self) {
        DB::destroy(&Options::default(), &self.path).unwrap_or_else(|err| {
            panic!("failed to destroy DB: {err}");
        });
    }
}

// implement for &DBPath (reference) instead of for DBPath (owned value)
// because we want to make sure the owned value lives until the end of its
// scope, so that the DB isn't destroyed prematurely.
impl AsRef<Path> for &TempDataDir {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
