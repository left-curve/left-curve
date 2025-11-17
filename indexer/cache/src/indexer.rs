use {
    crate::{Context, cache_file::CacheFile, indexer_path::IndexerPath},
    grug_types::BlockAndBlockOutcomeWithHttpDetails,
    std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Arc, Mutex},
    },
};

#[cfg(feature = "http-request-details")]
use grug_types::{Hash256, HttpRequestDetails};

#[derive(Default)]
pub struct Cache {
    pub context: Context,
    blocks: Arc<Mutex<HashMap<u64, BlockAndBlockOutcomeWithHttpDetails>>>,
}

impl Cache {
    pub fn new_with_tempdir() -> Self {
        Self {
            context: Context {
                indexer_path: IndexerPath::new_with_tempdir(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn new_with_dir(directory: PathBuf) -> Self {
        Self {
            context: Context {
                indexer_path: IndexerPath::new_with_dir(directory),
                ..Default::default()
            },
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
}

impl grug_app::Indexer for Cache {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        // TODO: create missing directories
        Ok(())
    }

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
            CacheFile::compress_file(file_path)?;
        }

        Ok(())
    }
}
