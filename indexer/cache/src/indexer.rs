use {
    crate::{Context, cache_file::CacheFile, error::Result, indexer_path::IndexerPath},
    async_trait::async_trait,
    grug_types::BlockAndBlockOutcomeWithHttpDetails,
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        io::Write,
        path::PathBuf,
        sync::{Arc, Mutex},
    },
};

#[cfg(feature = "s3")]
use {
    crate::error::IndexerError,
    crate::s3,
    roaring::RoaringTreemap,
    std::fs::File,
    std::io::{BufReader, BufWriter},
    std::time::Duration,
    std::time::Instant,
    tokio::{sync::Semaphore, task::JoinSet, time::sleep},
};

#[cfg(feature = "http-request-details")]
use grug_types::{Hash256, HttpRequestDetails};

const HIGHEST_BLOCK_FILENAME: &str = "last_block.json";

#[cfg(feature = "s3")]
const S3_HIGHEST_BLOCK_FILENAME: &str = "s3_highest_block.json";

#[cfg(feature = "s3")]
const S3_BLOCKS_FILENAME: &str = "s3_blocks.bin";

#[cfg(feature = "s3")]
const MAX_CONCURRENT_S3_UPLOADS: usize = 100;

#[cfg(feature = "s3")]
const INTERVAL_BETWEEN_S3_BITMAP_STORES: u64 = 60;

// TODO: need to add `keep_blocks` configuration to allow choosing if we keep blocks
// or not, to save disk space. `app.toml` could also add a u64 field to limit the
// number of blocks to keep, deleting the oldest ones when exceeding that number.

#[derive(Default)]
pub struct Cache {
    pub context: Context,
    // This because the way indexer methods are called, we need to store the blocks
    // in memory between `pre_indexing`, `index_block` and `post_indexing`.
    blocks: Arc<Mutex<HashMap<u64, BlockAndBlockOutcomeWithHttpDetails>>>,
    #[cfg(feature = "s3")]
    pub s3_bitmap: Arc<Mutex<RoaringTreemap>>,
}

#[derive(Serialize, Deserialize)]
struct LastBlockHeight {
    block_height: u64,
}

impl Cache {
    pub fn new_with_tempdir() -> Self {
        let indexer_path = IndexerPath::new_with_tempdir();

        #[cfg(feature = "s3")]
        let s3_bitmap = Arc::new(Mutex::new(Self::s3_bitmap(&indexer_path)));

        Self {
            context: Context {
                indexer_path,
                ..Default::default()
            },
            #[cfg(feature = "s3")]
            s3_bitmap,
            ..Default::default()
        }
    }

    pub fn new_with_dir(directory: PathBuf) -> Self {
        let indexer_path = IndexerPath::new_with_dir(directory);

        #[cfg(feature = "s3")]
        let s3_bitmap = Arc::new(Mutex::new(Self::s3_bitmap(&indexer_path)));

        Self {
            context: Context {
                indexer_path,
                ..Default::default()
            },
            #[cfg(feature = "s3")]
            s3_bitmap,
            ..Default::default()
        }
    }

    /// Set HTTP request details for transactions in the given block, those details
    /// are previously stored in the context by the httpd
    #[cfg(feature = "http-request-details")]
    fn set_http_request_details(
        &self,
        block: &grug_types::Block,
    ) -> grug_app::IndexerResult<HashMap<Hash256, HttpRequestDetails>> {
        let mut http_request_details: HashMap<Hash256, HttpRequestDetails> = HashMap::new();

        let mut transaction_hash_details = self
            .context
            .transactions_http_request_details
            .lock()
            .map_err(|_| grug_app::IndexerError::mutex_poisoned())?;

        http_request_details.extend(block.txs.iter().filter_map(|tx| {
            transaction_hash_details
                .remove(&tx.1)
                .map(|details| (tx.1, details))
        }));

        transaction_hash_details.clean();

        #[cfg(feature = "metrics")]
        metrics::gauge!("indexer.http_request_details.total")
            .set(transaction_hash_details.len() as f64);

        drop(transaction_hash_details);

        Ok(http_request_details)
    }

    #[cfg(feature = "s3")]
    fn s3_block_file_path(indexer_path: &IndexerPath) -> PathBuf {
        indexer_path.blocks_path().join(S3_BLOCKS_FILENAME)
    }

    #[cfg(feature = "s3")]
    pub fn s3_bitmap(indexer_path: &IndexerPath) -> RoaringTreemap {
        if let Ok(file) = File::open(Self::s3_block_file_path(indexer_path)) {
            RoaringTreemap::deserialize_from(BufReader::new(file)).unwrap()
        } else {
            RoaringTreemap::new()
        }
    }

    #[cfg(feature = "s3")]
    pub fn store_bitmap(context: &Context, s3_bitmap: Arc<Mutex<RoaringTreemap>>) -> Result<()> {
        #[cfg(feature = "tracing")]
        let time_start = Instant::now();

        let s3_bitmap = s3_bitmap
            .lock()
            .map_err(|err| IndexerError::mutex_poisoned(err.to_string()))?;
        #[cfg(feature = "tracing")]
        let blocks_len = s3_bitmap.len();

        let mut tmp = tempfile::NamedTempFile::new_in(context.indexer_path.blocks_path())?;
        s3_bitmap.serialize_into(BufWriter::new(&mut tmp))?;
        drop(s3_bitmap);

        tmp.flush()?;
        tmp.persist(Self::s3_block_file_path(&context.indexer_path))?;

        #[cfg(feature = "tracing")]
        tracing::info!(time_elapsed = ?time_start.elapsed(), blocks_len, "Stored S3 bitmap to disk");

        Ok(())
    }

    /// Store the last block height in the file cache.
    fn store_last_block_height(context: &Context, block_height: u64, filename: &str) -> Result<()> {
        // We don't store if existing block height is greater
        if let Some(existing_block_height) = Self::read_last_block_height(context, filename)?
            && existing_block_height >= block_height
        {
            return Ok(());
        }

        let dir = context.indexer_path.blocks_path();
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        let payload = LastBlockHeight { block_height };
        serde_json::to_writer(&mut tmp, &payload)?;
        tmp.flush()?;
        tmp.persist(context.indexer_path.blocks_path().join(filename))?;

        Ok(())
    }

    /// Read the last block height from the file cache.
    fn read_last_block_height(context: &Context, filename: &str) -> Result<Option<u64>> {
        let path = context.indexer_path.blocks_path().join(filename);

        if !path.exists() {
            return Ok(None);
        }

        let file = std::fs::File::open(path)?;
        let payload: LastBlockHeight = serde_json::from_reader(file)?;

        Ok(Some(payload.block_height))
    }

    /// Read the last S3 synced block height from the file cache.
    #[cfg(feature = "s3")]
    pub fn read_last_s3_block_height(context: &Context) -> Result<Option<u64>> {
        Self::read_last_block_height(context, S3_HIGHEST_BLOCK_FILENAME)
    }

    /// Store the last S3 synced block height in the file cache.
    #[cfg(feature = "s3")]
    pub fn store_last_s3_block_height(context: &Context, height: u64) -> Result<()> {
        Self::store_last_block_height(context, height, S3_HIGHEST_BLOCK_FILENAME)
    }

    #[cfg(feature = "s3")]
    async fn sync_block_to_s3(
        context: &Context,
        s3_bitmap: Arc<Mutex<RoaringTreemap>>,
        block_height: u64,
    ) -> Result<()> {
        {
            let s3_bitmap = s3_bitmap
                .lock()
                .map_err(|err| IndexerError::mutex_poisoned(err.to_string()))?;
            if s3_bitmap.contains(block_height) {
                return Ok(());
            }
        }

        let blocks_root = context.indexer_path.blocks_path();
        let s3_client = s3::Client::new(context.s3.clone()).await?;

        let block_path = context.indexer_path.block_path(block_height);
        let file_path = CacheFile::file_path(block_path);

        let mut s3_key = file_path
            .strip_prefix(&blocks_root)
            .map(|rel| rel.to_string_lossy().into_owned())?;

        // Prepend optional path/prefix within the bucket
        if !context.s3.path.is_empty() {
            let prefix = context.s3.path.trim_matches('/');
            if !prefix.is_empty() {
                s3_key = format!("{}/{}", prefix, s3_key.trim_start_matches('/'));
            }
        }

        #[cfg(feature = "tracing")]
        let path = file_path.clone();

        // Outer retry loop on top of the SDK's own retry budget (configured
        // in `s3::S3Config::client`, currently 3 attempts). Three outer
        // attempts × ~30s SDK ceiling × 100 concurrent blocks is already a
        // worst case of several minutes when S3 is unreachable; eleven outer
        // attempts × 1s sleep was on top of that and turned a single bad
        // bucket into a half-hour-long process. Exponential backoff (200ms,
        // 400ms, 800ms) instead of a flat 1s sleep so we recover faster from
        // transient blips without hammering on persistent failure.
        for attempt in 0u32..3 {
            // When restarting the node, we could have some already copied over blocks. Skipping those.
            match s3_client.exists(&s3_key).await {
                Ok(false) => {
                    // File does not exist in S3, proceed with upload
                },
                Ok(true) => {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        block_height,
                        key = %s3_key,
                        path = %path.display(),
                        "Cached block already exists in S3"
                    );

                    // Mark block as uploaded in the bitmap so we don't check S3 again
                    {
                        let mut s3_bitmap = s3_bitmap
                            .lock()
                            .map_err(|err| IndexerError::mutex_poisoned(err.to_string()))?;
                        s3_bitmap.insert(block_height);
                    }

                    return Ok(());
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        block_height,
                        key = %s3_key,
                        path = %path.display(),
                        error = %_err,
                        "Failed to check if cached block exists in S3"
                    );

                    // Error occurred, proceed with upload
                },
            }

            #[cfg(any(feature = "tracing", feature = "metrics"))]
            let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
            #[cfg(any(feature = "tracing", feature = "metrics"))]
            let start = Instant::now();

            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height,
                key = %s3_key,
                path = %path.display(),
                "Uploading cached block to S3"
            );

            #[cfg(feature = "metrics")]
            metrics::counter!("indexer.s3.upload.attempts").increment(1);

            match s3_client.upload_file(s3_key.clone(), &file_path).await {
                Ok(()) => {
                    #[cfg(any(feature = "tracing", feature = "metrics"))]
                    let elapsed = start.elapsed();

                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        block_height,
                        bytes = size,
                        ms = %elapsed.as_millis(),
                        file = %file_path.display(),
                        "S3 upload succeeded"
                    );
                    #[cfg(feature = "metrics")]
                    {
                        metrics::counter!("indexer.s3.upload.success").increment(1);
                        metrics::histogram!("indexer.s3.upload.bytes").record(size as f64);
                        metrics::histogram!("indexer.s3.upload.duration")
                            .record(elapsed.as_secs_f64());
                    }

                    // Mark block as uploaded in the bitmap
                    {
                        let mut s3_bitmap = s3_bitmap
                            .lock()
                            .map_err(|err| IndexerError::mutex_poisoned(err.to_string()))?;
                        s3_bitmap.insert(block_height);
                    }

                    return Ok(());
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_err, file = %file_path.display(), "S3 upload failed");
                    #[cfg(feature = "metrics")]
                    metrics::counter!("indexer.s3.upload.failure").increment(1);

                    sleep(Duration::from_millis(200u64 << attempt)).await;
                },
            }
        }

        // All retries failed
        Err(IndexerError::s3upload_failed(block_height, s3_key))
    }

    #[cfg(feature = "s3")]
    pub async fn sync_to_s3(
        context: &Context,
        s3_bitmap: Arc<Mutex<RoaringTreemap>>,
        last_synced_height: u64,
    ) -> Result<Option<u64>> {
        if !context.s3.enabled {
            return Ok(None);
        }

        let Some(last_stored_height) =
            Self::read_last_block_height(context, HIGHEST_BLOCK_FILENAME)?
        else {
            #[cfg(feature = "tracing")]
            tracing::debug!("No blocks stored locally, skipping S3 sync");

            return Ok(None);
        };

        if last_synced_height >= last_stored_height {
            // Already up-to-date
            return Ok(None);
        }

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_S3_UPLOADS));
        // Use a `JoinSet` instead of collecting `JoinHandle`s into a `Vec` and
        // joining via `try_join_all`. Two reasons, both about cancellation:
        //
        // 1. `try_join_all` short-circuits on the first `Err`, but the dropped
        //    `JoinHandle`s for the other in-flight uploads do NOT get
        //    cancelled — they keep running on the runtime, wasting work
        //    against a bucket we already know is broken.
        // 2. Dropping a `JoinSet` aborts all of its tasks. That's what makes
        //    an outer `tokio::time::timeout(...)` actually free resources
        //    rather than leak them. The CLI handler relies on this for the
        //    wallclock timeout.
        //
        // We also `set.shutdown().await` on the first error so siblings stop
        // immediately rather than running until natural completion.
        let mut set: JoinSet<Result<()>> = JoinSet::new();

        for block_height in (last_synced_height + 1)..=last_stored_height {
            let context = context.clone();
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("Semaphore should not be acquired");
            let s3_bitmap = s3_bitmap.clone();

            set.spawn(async move {
                let _permit = permit;
                Self::sync_block_to_s3(&context, s3_bitmap, block_height).await
            });
        }

        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(Ok(())) => {},
                Ok(Err(err)) => {
                    set.shutdown().await;
                    return Err(err);
                },
                Err(join_err) => {
                    set.shutdown().await;
                    return Err(join_err.into());
                },
            }
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            from_height = last_synced_height + 1,
            to_height = last_stored_height,
            "S3 sync up-to-date"
        );

        Ok(Some(last_stored_height))
    }
}

#[async_trait]
impl grug_app::Indexer for Cache {
    async fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "s3")]
        if self.context.s3.enabled {
            let context = self.context.clone();
            let s3_bitmap = self.s3_bitmap.clone();
            let mut start_time = Instant::now();

            tokio::spawn(async move {
                loop {
                    let last_synced_height = match Self::read_last_block_height(
                        &context,
                        S3_HIGHEST_BLOCK_FILENAME,
                    ) {
                        Err(_error) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!(error = %_error, "Failed to read last S3 synced block height");

                            sleep(Duration::from_millis(1000)).await;
                            continue;
                        },
                        Ok(last_synced_height) => last_synced_height.unwrap_or(0),
                    };

                    match Self::sync_to_s3(&context, s3_bitmap.clone(), last_synced_height).await {
                        Err(_error) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!(error = %_error, "Failed to sync S3");
                        },
                        Ok(Some(last_stored_height)) => {
                            if let Err(_err) = Self::store_last_block_height(
                                &context,
                                last_stored_height,
                                S3_HIGHEST_BLOCK_FILENAME,
                            ) {
                                #[cfg(feature = "tracing")]
                                tracing::error!(error = %_err, "Failed to store last synced block height");
                            }
                        },
                        Ok(None) => {
                            // Already synced
                        },
                    }

                    // Periodically store the bitmap to disk
                    if start_time.elapsed().as_secs() > INTERVAL_BETWEEN_S3_BITMAP_STORES {
                        if let Err(_error) = Self::store_bitmap(&context, s3_bitmap.clone()) {
                            #[cfg(feature = "tracing")]
                            tracing::error!(
                                error = %_error,
                                "Failed to store S3 bitmap to disk",
                            );
                        }

                        start_time = Instant::now();
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            });
        }

        // NOTE: might need to create caching directory, but working so far.
        Ok(())
    }

    #[cfg(feature = "s3")]
    async fn shutdown(&mut self) -> grug_app::IndexerResult<()> {
        Self::store_bitmap(&self.context, self.s3_bitmap.clone())?;

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn pre_indexing(
        &self,
        block_height: u64,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        let file_path = self.context.indexer_path.block_path(block_height);

        // This is used when reindexing existing blocks, since `index_block` won't be called.
        if CacheFile::exists(file_path.clone()) {
            let cache_file = CacheFile::load_from_disk(file_path)?;

            self.blocks
                .lock()
                .map_err(|_| grug_app::IndexerError::mutex_poisoned())?
                .insert(block_height, cache_file.data.clone());

            ctx.insert(cache_file.data);
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        let file_path = self.context.indexer_path.block_path(block.info.height);

        if CacheFile::exists(file_path.clone()) {
            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = block.info.height,
                "Block already cached, skipping writing to disk",
            );

            // I have to reload the file to get the http-request-details,
            // which we lost since if we are not going through the httpd.
            let cache_file = CacheFile::load_from_disk(file_path)?;

            self.blocks
                .lock()
                .map_err(|_| grug_app::IndexerError::mutex_poisoned())?
                .insert(block.info.height, cache_file.data.clone());

            ctx.insert(cache_file.data);
        } else {
            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = block.info.height,
                file_path = ?file_path,
                "Block will be saved to disk",
            );

            #[allow(unused_mut)]
            let mut cache_file = CacheFile::new(file_path, block.clone(), block_outcome.clone());

            #[cfg(feature = "http-request-details")]
            {
                cache_file.data.http_request_details = self.set_http_request_details(block)?;
            }
            cache_file.save_to_disk()?;

            self.blocks
                .lock()
                .map_err(|_| grug_app::IndexerError::mutex_poisoned())?
                .insert(block.info.height, cache_file.data.clone());

            ctx.insert(cache_file.data);
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn post_indexing(
        &self,
        block_height: u64,
        _cfg: grug_types::Config,
        _app_cfg: grug_types::Json,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        let Some(data) = self
            .blocks
            .lock()
            .map_err(|_| grug_app::IndexerError::mutex_poisoned())?
            .remove(&block_height)
        else {
            return Err(grug_app::IndexerError::hook(format!(
                "Block data for height {block_height} not found in cache indexer",
            )));
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            block_height,
            "Added block data to indexer context in post_indexing",
        );

        ctx.insert(data);

        let file_path = self.context.indexer_path.block_path(block_height);

        if CacheFile::exists(file_path.clone()) {
            CacheFile::compress_file(file_path.clone())?;

            Self::store_last_block_height(&self.context, block_height, HIGHEST_BLOCK_FILENAME)?;
        }

        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(all(test, feature = "s3"))]
mod tests {
    use {super::*, crate::S3Config};

    /// Regression test for the production hang on 2026-05-03: with an
    /// unreachable S3 endpoint, `sync_to_s3` must return `Err` quickly
    /// rather than running indefinitely. If this ever takes >15s the
    /// pile-up bug is back. The outer `tokio::time::timeout` is the
    /// safety net so a regression fails the test rather than hangs CI.
    ///
    /// We point the SDK at `127.0.0.1:1` so connect attempts are refused
    /// immediately (no DNS, no real wait) — this exercises the
    /// SDK-timeout / outer-retry / `JoinSet` chain end to end without
    /// requiring any network. Happy-path runtime is ~3s; the 15s ceiling
    /// is ~4× margin and caps CI cost on regression.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn sync_to_s3_fails_fast_when_s3_unreachable() {
        let cache = Cache {
            context: Context {
                s3: S3Config {
                    enabled: true,
                    path: "test/".to_string(),
                    endpoint: "http://127.0.0.1:1".to_string(),
                    access_key: "access".to_string(),
                    secret_key: "secret".to_string(),
                    bucket: "test-bucket".to_string(),
                    region: "us-east-1".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        cache
            .context
            .indexer_path
            .create_dirs_if_needed()
            .expect("create blocks dir");

        // Put one block on disk and record it as the highest stored height,
        // so `sync_to_s3` finds work to do and actually exercises the S3
        // client.
        let block_height: u64 = 1;

        {
            let block_file_path =
                CacheFile::file_path(cache.context.indexer_path.block_path(block_height));

            std::fs::create_dir_all(block_file_path.parent().expect("block parent dir"))
                .expect("create block subdir");
            std::fs::write(&block_file_path, b"placeholder").expect("write block file");

            Cache::store_last_block_height(&cache.context, block_height, HIGHEST_BLOCK_FILENAME)
                .expect("store last block height")
        };

        let result = tokio::time::timeout(
            Duration::from_secs(15),
            Cache::sync_to_s3(&cache.context, cache.s3_bitmap.clone(), 0),
        )
        .await;

        match result {
            Err(_elapsed) => {
                panic!("sync_to_s3 hung past 15s — fail-fast regression is back");
            },
            Ok(Ok(_)) => {
                panic!("sync_to_s3 unexpectedly succeeded against a dead endpoint");
            },
            // The dead endpoint causes `sync_block_to_s3` to exhaust
            // its outer retry budget and return `S3UploadFailed`. The
            // `JoinSet` arm then propagates that exact error out of
            // `sync_to_s3`. With only one block in the test
            // (`block_height = 1`), the error must reference that
            // block — anything else means we're failing for a reason
            // other than what this test is supposed to cover.
            Ok(Err(IndexerError::S3UploadFailed {
                block_height: failed_height,
                ..
            })) => {
                assert_eq!(failed_height, block_height, "wrong block in S3UploadFailed");
            },
            Ok(Err(other)) => {
                panic!("expected IndexerError::S3UploadFailed, got: {other:?}")
            },
        }
    }
}
