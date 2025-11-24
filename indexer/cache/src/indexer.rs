use {
    crate::{
        Context, cache_file::CacheFile, error::Result, indexer_path::IndexerPath,
        runtime::RuntimeHandler,
    },
    grug_types::BlockAndBlockOutcomeWithHttpDetails,
    serde::Serialize,
    std::{
        collections::HashMap,
        io::Write,
        path::PathBuf,
        sync::{Arc, Mutex},
    },
};

#[cfg(feature = "s3")]
use {crate::s3, std::time::Duration, std::time::Instant, tokio::time::sleep};

#[cfg(feature = "http-request-details")]
use grug_types::{Hash256, HttpRequestDetails};

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
}

#[derive(Serialize)]
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

    fn store_last_block_height(&self, block_height: u64) -> Result<()> {
        let dir = self.context.indexer_path.blocks_path();
        let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
        let payload = LastBlockHeight { block_height };
        serde_json::to_writer(&mut tmp, &payload)?;
        tmp.flush()?;
        tmp.persist(self.context.indexer_path.blocks_path().join("last.json"))
            .map(|_| ())
            .map_err(|e| e.error.into())
    }
}

impl grug_app::Indexer for Cache {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
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
            let _compressed_path = CacheFile::compress_file(file_path.clone())?;

            self.store_last_block_height(block_height)?;

            // If S3 config present, upload compressed file in background
            #[cfg(feature = "s3")]
            if self.context.s3.enabled {
                let cfg = self.context.s3.clone();
                let blocks_root = self.context.indexer_path.blocks_path();
                // Derive an S3 key relative to blocks/ directory
                let mut key = match _compressed_path.strip_prefix(&blocks_root) {
                    Ok(rel) => rel.to_string_lossy().into_owned(),
                    Err(_) => {
                        // Fallback to expected relative structure using block height
                        let mut rel_path = self.context.indexer_path.block_path(block_height);
                        rel_path.set_extension("borsh.xz");
                        rel_path
                            .strip_prefix(&blocks_root)
                            .unwrap_or(&rel_path)
                            .to_string_lossy()
                            .into_owned()
                    },
                };

                // Prepend optional path/prefix within the bucket
                if !cfg.path.is_empty() {
                    let prefix = cfg.path.trim_matches('/');
                    if !prefix.is_empty() {
                        key = format!("{}/{}", prefix, key.trim_start_matches('/'));
                    }
                }

                let path = _compressed_path.clone();

                #[cfg(feature = "tracing")]
                tracing::info!(
                    block_height,
                    key = %key,
                    path = %path.display(),
                    "Uploading cached block to S3"
                );

                self.runtime_handler.block_on(async move {
                    #[cfg(feature = "metrics")]
                    metrics::counter!("indexer.s3.upload.attempts").increment(1);

                    // Retry logic, in case of network error.
                    for _ in 0..=10 {
                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        let start = Instant::now();
                        match s3::upload_file(cfg.clone(), key.clone(), &path).await {
                            Ok(()) => {
                                let elapsed = start.elapsed();
                                #[cfg(feature = "tracing")]
                                tracing::info!(
                                    block_height,
                                    bytes = size,
                                    ms = %elapsed.as_millis(),
                                    file = %path.display(),
                                    "S3 upload succeeded"
                                );
                                #[cfg(feature = "metrics")]
                                {
                                    metrics::counter!("indexer.s3.upload.success").increment(1);
                                    metrics::histogram!("indexer.s3.upload.bytes").record(size as f64);
                                    metrics::histogram!("indexer.s3.upload.duration")
                                        .record(elapsed.as_secs_f64());
                                }

                                break;
                            },
                            Err(err) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!(error = %err, file = %path.display(), "S3 upload failed");
                                #[cfg(feature = "metrics")]
                                metrics::counter!("indexer.s3.upload.failure").increment(1);

                                sleep(Duration::from_secs(1)).await;
                            },
                        }
                    }
                });
            }
        }

        Ok(())
    }
}
