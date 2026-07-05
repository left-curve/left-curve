//! Retrieval clients for cached blocks stored in S3/B2.
//!
//! Layered on top of the general [`crate::s3::Client`], which does the raw
//! transport (connect + `get(key)`):
//!
//! - [`BlockClient`] retrieves individual blocks — one object per block under
//!   the reverse-digit key layout, payloads in legacy LZMA1 (`.borsh.xz`).
//! - [`BatchClient`] retrieves batch archives — a contiguous range of blocks
//!   stored as one `<start>-<end>.tar.xz` (xz-compressed tar of `<height>.borsh`).
//!
//! Both wrap the same B2 client and share [`StorageConfig`] for where/how the
//! result is written to disk.

use {
    crate::{
        error::{IndexerError, Result},
        indexer_path::IndexerPath,
        s3::Client,
    },
    lzma_rs::{lzma_decompress, xz_decompress},
    std::path::PathBuf,
    tar::Archive,
};

/// Where and how retrieved blocks are written to disk.
#[derive(Clone, Debug)]
pub struct StorageConfig {
    /// Directory to write retrieved files into (created if missing).
    pub dir: PathBuf,
    /// Decompress after download: single blocks `.borsh.xz` -> `.borsh`;
    /// batches `.tar.xz` -> extracted `.borsh` entries.
    pub decompress: bool,
    /// Batches only: delete the downloaded `.tar.xz` after extracting it.
    /// Ignored when `decompress` is false (nothing has been extracted yet).
    pub remove_archive: bool,
}

// ---- single-block client ----

/// Retrieves individual blocks stored one object per block under the
/// reverse-digit key layout (`<prefix>/<rev-digit dirs>/<height>.borsh.xz`).
pub struct BlockClient {
    b2: Client,
    prefix: String,
    storage: StorageConfig,
}

impl BlockClient {
    /// `prefix` is the key prefix the blocks live under, e.g.
    /// `"dango/indexer/blocks/"`.
    pub fn new(b2: Client, prefix: impl Into<String>, storage: StorageConfig) -> Self {
        Self {
            b2,
            prefix: prefix.into(),
            storage,
        }
    }

    /// S3 key for a block height, matching the layout the node writes.
    fn key(&self, height: u64) -> Result<String> {
        // Reuse the node's own path logic so the key always matches. The base
        // dir is irrelevant — only the suffix below `blocks/` is used.
        let ip = IndexerPath::new_with_dir(PathBuf::new());
        let root = ip.blocks_path();
        let rel = ip.block_path(height);
        let rel = rel.strip_prefix(&root)?;
        let rel_key = format!("{}.borsh.xz", rel.to_string_lossy());

        Ok(if self.prefix.is_empty() {
            rel_key
        } else {
            format!(
                "{}/{}",
                self.prefix.trim_matches('/'),
                rel_key.trim_start_matches('/')
            )
        })
    }

    /// Download a single block into the configured directory. Returns the path
    /// written (`.borsh` if decompressing, else `.borsh.xz`), or `None` if the
    /// block is not in the bucket.
    pub async fn get_block(&self, height: u64) -> Result<Option<PathBuf>> {
        let key = self.key(height)?;
        let Some(bytes) = self.b2.get(&key).await? else {
            return Ok(None);
        };

        std::fs::create_dir_all(&self.storage.dir)?;

        let path = if self.storage.decompress {
            let mut raw = Vec::new();
            lzma_decompress(&mut bytes.as_slice(), &mut raw)
                .map_err(|e| IndexerError::byte_stream(e.to_string()))?;
            let path = self.storage.dir.join(format!("{height}.borsh"));
            std::fs::write(&path, &raw)?;
            path
        } else {
            let path = self.storage.dir.join(format!("{height}.borsh.xz"));
            std::fs::write(&path, &bytes)?;
            path
        };

        Ok(Some(path))
    }
}

// ---- batch client ----

/// Retrieves batch archives, where a contiguous range of blocks is stored as one
/// `<prefix>/<start>-<end>.tar.xz` object.
pub struct BatchClient {
    b2: Client,
    prefix: String,
    batch_size: u64,
    storage: StorageConfig,
}

impl BatchClient {
    /// `prefix` is the key prefix for batch objects, e.g. `"batches/v1/"`.
    /// `batch_size` is the number of blocks per batch (must match the producer).
    pub fn new(
        b2: Client,
        prefix: impl Into<String>,
        batch_size: u64,
        storage: StorageConfig,
    ) -> Self {
        Self {
            b2,
            prefix: prefix.into(),
            batch_size,
            storage,
        }
    }

    /// The `[start, end)` range that contains `height`.
    pub fn range_for_height(&self, height: u64) -> (u64, u64) {
        let start = height - height % self.batch_size;
        (start, start + self.batch_size)
    }

    /// S3 key for a batch range.
    fn key(&self, start: u64, end: u64) -> String {
        let name = format!("{start}-{end}.tar.xz");
        if self.prefix.is_empty() {
            name
        } else {
            format!("{}/{}", self.prefix.trim_matches('/'), name)
        }
    }

    /// Download the batch that contains `height`.
    pub async fn get_batch_for_height(&self, height: u64) -> Result<Option<PathBuf>> {
        let (start, end) = self.range_for_height(height);
        self.get_batch(start, end).await
    }

    /// Download the batch for `[start, end)`. With `decompress`, extracts the
    /// `<height>.borsh` entries into the configured dir and returns that dir;
    /// otherwise stores the `.tar.xz` and returns its path. `None` if absent.
    pub async fn get_batch(&self, start: u64, end: u64) -> Result<Option<PathBuf>> {
        let key = self.key(start, end);
        let Some(bytes) = self.b2.get(&key).await? else {
            return Ok(None);
        };

        std::fs::create_dir_all(&self.storage.dir)?;
        let archive_path = self.storage.dir.join(format!("{start}-{end}.tar.xz"));
        std::fs::write(&archive_path, &bytes)?;

        if !self.storage.decompress {
            return Ok(Some(archive_path));
        }

        // xz -> tar -> extract the `<height>.borsh` entries into `dir`.
        let mut tar_bytes = Vec::new();
        xz_decompress(&mut bytes.as_slice(), &mut tar_bytes)
            .map_err(|e| IndexerError::byte_stream(e.to_string()))?;
        Archive::new(tar_bytes.as_slice()).unpack(&self.storage.dir)?;

        if self.storage.remove_archive {
            let _ = std::fs::remove_file(&archive_path);
        }

        Ok(Some(self.storage.dir.clone()))
    }
}
