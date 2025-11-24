use {
    crate::{
        Context, cache_file::CacheFile, error::Result, indexer_path::IndexerPath,
        runtime::RuntimeHandler,
    },
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
    crate::error::IndexerError, crate::s3, std::time::Duration, std::time::Instant,
    tokio::time::sleep,
};

#[cfg(feature = "http-request-details")]
use grug_types::{Hash256, HttpRequestDetails};

#[cfg(feature = "s3")]
const S3_HIGHEST_BLOCK_FILENAME: &str = "s3_highest_block.json";
const HIGHEST_BLOCK_FILENAME: &str = "last_block.json";

// TODO: need to add `keep_blocks` configuration to allow choosing if we keep blocks
// or not, to save disk space. `app.toml` could also add a u64 field to limit the
// number of blocks to keep, deleting the oldest ones when exceeding that number.

#[derive(Default)]
pub struct Cache {
    pub context: Context,
    pub runtime_handler: RuntimeHandler,
    // This because the way indexer methods are called, we need to store the blocks
    // in memory between `pre_indexing`, `index_block` and `post_indexing`.
    blocks: Arc<Mutex<HashMap<u64, BlockAndBlockOutcomeWithHttpDetails>>>,
    // This ensures all blocks are synced before writing cache
    #[cfg(feature = "s3")]
    s3_block_heights: Arc<Mutex<HashMap<u64, bool>>>,
}

#[derive(Serialize, Deserialize)]
struct LastBlockHeight {
    block_height: u64,
}

impl Cache {
    pub fn new_with_tempdir() -> Self {
        Self {
            context: Context {
                indexer_path: IndexerPath::new_with_tempdir(),
                ..Default::default()
            },
            runtime_handler: RuntimeHandler::default(),
            ..Default::default()
        }
    }

    pub fn new_with_dir(directory: PathBuf) -> Self {
        Self {
            context: Context {
                indexer_path: IndexerPath::new_with_dir(directory),
                ..Default::default()
            },
            runtime_handler: RuntimeHandler::default(),
            ..Default::default()
        }
    }

    pub fn new_with_dir_and_runtime(directory: PathBuf, runtime_handler: RuntimeHandler) -> Self {
        Self {
            context: Context {
                indexer_path: IndexerPath::new_with_dir(directory),
                ..Default::default()
            },
            runtime_handler,
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

    /// Store the last block height in the cache.
    fn store_last_block_height(&self, block_height: u64, filename: &str) -> Result<()> {
        // We don't store if existing block height is greater
        if let Some(existing_block_height) = self.read_last_block_height(filename)?
            && existing_block_height >= block_height
        {
            return Ok(());
        }

        let dir = self.context.indexer_path.blocks_path();
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        let payload = LastBlockHeight { block_height };
        serde_json::to_writer(&mut tmp, &payload)?;
        tmp.flush()?;
        tmp.persist(self.context.indexer_path.blocks_path().join(filename))
            .map(|_| ())
            .map_err(|e| e.error.into())
    }

    /// Read the last block height from the cache.
    fn read_last_block_height(&self, filename: &str) -> Result<Option<u64>> {
        let path = self.context.indexer_path.blocks_path().join(filename);

        if !path.exists() {
            return Ok(None);
        }

        let file = std::fs::File::open(path)?;
        let payload: LastBlockHeight = serde_json::from_reader(file)?;

        Ok(Some(payload.block_height))
    }

    #[cfg(feature = "s3")]
    fn sync_to_s3(&self, from_height: u64, to_height: u64) -> Result<()> {
        if !self.context.s3.enabled {
            return Ok(());
        }

        let blocks_root = self.context.indexer_path.blocks_path();
        let s3_cfg = self.context.s3.clone();

        self.runtime_handler.block_on(async move {
            let s3_client = s3::Client::new(s3_cfg.clone()).await?;

            // NOTE: for simplification I don't do those uploads in parallel,
            // we could easily do it but need to handle errors properly like:
            //
            // - Do not write the highest block height locally if any failed
            // - Have a maximum number of concurrent uploads
            //
            for block_height in from_height..=to_height {
                let cfg = s3_cfg.clone();

                let block_path = self.context.indexer_path.block_path(block_height);
                let file_path = CacheFile::file_path(block_path);

                let mut s3_key = file_path
                    .strip_prefix(&blocks_root)
                    .map(|rel| rel.to_string_lossy().into_owned())?;

                // Prepend optional path/prefix within the bucket
                if !cfg.path.is_empty() {
                    let prefix = cfg.path.trim_matches('/');
                    if !prefix.is_empty() {
                        s3_key = format!("{}/{}", prefix, s3_key.trim_start_matches('/'));
                    }
                }

                let path = file_path.clone();

                // When restarting the node, we could have some already copied over blocks. Skipping those.
                match s3_client.exists(&s3_key).await {
                    Ok(false) => {
                        // File does not exist in S3, proceed with upload
                    },
                    Ok(true) => {
                        #[cfg(feature = "tracing")]
                        tracing::info!(
                            block_height,
                            key = %s3_key,
                            path = %path.display(),
                            "Cached block already exists in S3"
                        );
                        continue;
                    }
                    Err(err) => {
                        #[cfg(feature = "tracing")]
                        tracing::error!(
                            block_height,
                            key = %s3_key,
                            path = %path.display(),
                            error = %err,
                            "Failed to check if cached block exists in S3"
                        );

                        // Error occurred, proceed with upload
                    }
                }

                #[cfg(feature = "tracing")]
                tracing::info!(
                    block_height,
                    key = %s3_key,
                    path = %path.display(),
                    "Uploading cached block to S3"
                );

                #[cfg(feature = "metrics")]
                metrics::counter!("indexer.s3.upload.attempts").increment(1);

                // Naive retries, in case of network error.
                for _ in 0..=10 {
                    let size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
                    let start = Instant::now();

                    match s3_client.upload_file(s3_key.clone(), &file_path).await {
                        Ok(()) => {
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

                            // We mark this block as uploaded
                            self
                                .s3_block_heights
                                .lock()
                                .map_err(|e| crate::error::IndexerError::mutex_poisoned(e.to_string()))?.insert(block_height, true);

                            break;
                        },
                        Err(err) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!(error = %err, file = %file_path.display(), "S3 upload failed");
                            #[cfg(feature = "metrics")]
                            metrics::counter!("indexer.s3.upload.failure").increment(1);

                            sleep(Duration::from_secs(1)).await;
                        },
                    }
                }
            }

            Ok::<(), IndexerError>(())
        })?;

        let last_synced_height = self.read_last_block_height(S3_HIGHEST_BLOCK_FILENAME)?;

        // This happens when the indexer is started for the first time,
        // we don't want to sync since genesis but we'll
        // start syncing from now
        // TODO: Implement a command to sync older existing blocks to ensure
        // 100% coverage of backed up blocks on S3
        let Some(last_synced_height) = last_synced_height else {
            self.store_last_block_height(to_height, S3_HIGHEST_BLOCK_FILENAME)?;

            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = to_height,
                "Started S3 syncing for the first time"
            );

            return Ok(());
        };

        let mut s3_block_heights = self
            .s3_block_heights
            .lock()
            .map_err(|e| crate::error::IndexerError::mutex_poisoned(e.to_string()))?;

        // Find the first missing block in the range
        if let Some(block_height) =
            ((last_synced_height + 1)..=to_height).find(|h| !s3_block_heights.contains_key(h))
        {
            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = block_height,
                last_synced_height = last_synced_height,
                to_height = to_height,
                all_blocks = s3_block_heights.len(),
                "S3 sync not up-to-date, missing blocks"
            );

            #[cfg(feature = "metrics")]
            metrics::gauge!("indexer.s3.blocks_cache").set(s3_block_heights.len() as u32);

            return Ok(());
        }

        // All blocks are present
        s3_block_heights.clear();

        #[cfg(feature = "metrics")]
        metrics::gauge!("indexer.s3.blocks_cache").set(0);

        drop(s3_block_heights);

        self.store_last_block_height(to_height, S3_HIGHEST_BLOCK_FILENAME)?;

        #[cfg(feature = "tracing")]
        tracing::info!("S3 sync up-to-date");

        Ok(())
    }
}

impl grug_app::Indexer for Cache {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "s3")]
        if self.context.s3.enabled {
            let last_s3_sync_height = self.read_last_block_height(S3_HIGHEST_BLOCK_FILENAME)?;
            let Some(last_s3_sync_height) = last_s3_sync_height else {
                return Ok(());
            };

            let last_written_height = self.read_last_block_height(HIGHEST_BLOCK_FILENAME)?;
            let Some(last_written_height) = last_written_height else {
                return Ok(());
            };

            if last_s3_sync_height < last_written_height {
                self.sync_to_s3(last_s3_sync_height, last_written_height)?;
            }
        }

        // NOTE: might need to create caching directory, but working so far.
        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    fn pre_indexing(
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
    fn index_block(
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
    fn post_indexing(
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

            self.store_last_block_height(block_height, HIGHEST_BLOCK_FILENAME)?;

            #[cfg(feature = "s3")]
            if self.context.s3.enabled {
                self.sync_to_s3(block_height, block_height)?;
            }
        }

        Ok(())
    }
}
