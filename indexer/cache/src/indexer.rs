use {
    crate::{Context, cache_file::CacheFile, indexer_path::IndexerPath},
    grug_types::{Hash256, HttpRequestDetails},
    std::collections::HashMap,
};

pub struct Cache {
    pub indexer_path: IndexerPath,
    pub context: Context,
}

impl Cache {
    pub fn new(indexer_path: IndexerPath) -> Self {
        Self {
            indexer_path,
            context: Default::default(),
        }
    }

    /// Set HTTP request details for transactions in the given block, those details
    /// are previously stored in the context by the httpd
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
    fn last_indexed_block_height(&self) -> grug_app::IndexerResult<Option<u64>> {
        todo!(
            "Implement last_indexed_block_height for Cache indexer, looking at the last cached block file on disk"
        )
    }

    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        // TODO: create missing directories
        Ok(())
    }

    fn shutdown(&mut self) -> grug_app::IndexerResult<()> {
        Ok(())
    }

    fn pre_indexing(
        &self,
        _block_height: u64,
        _ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        // TODO: if file exists on disk, load them to memory and insert into ctx

        Ok(())
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        let mut cache_file = CacheFile::new(
            self.indexer_path.block_path(block.info.height),
            block.clone(),
            block_outcome.clone(),
        );

        #[cfg(feature = "http-request-details")]
        {
            cache_file.data.transactions_http_request_details =
                self.set_http_request_details(block)?;
        }

        cache_file.save_to_disk()?;

        ctx.insert(cache_file.data.clone());

        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        let block_filename = self.indexer_path.block_path(block_height);

        todo!("Store block and block outcome in ctx")
    }

    fn wait_for_finish(&self) -> grug_app::IndexerResult<()> {
        Ok(())
    }
}
