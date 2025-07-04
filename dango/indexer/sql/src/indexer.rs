use {
    crate::context::Context,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    grug::Storage,
    grug_app::QuerierProvider,
    indexer_sql::{block_to_index::BlockToIndex, non_blocking_indexer::RuntimeHandler},
    std::sync::Arc,
};
#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

mod accounts;
mod transfers;

pub struct Indexer {
    pub runtime_handle: RuntimeHandler,
    pub context: Context,
}

impl grug_app::Indexer for Indexer {
    fn start(&mut self, _storage: &dyn Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        self.runtime_handle.block_on(async {
            Migrator::up(&self.context.db, None)
                .await
                .map_err(|e| grug_app::IndexerError::Database(e.to_string()))?;

            Ok::<(), grug_app::IndexerError>(())
        })?;

        #[cfg(feature = "metrics")]
        {
            transfers::init_metrics();
            accounts::init_metrics();
            init_metrics();

            histogram!("indexer.dango.start.duration",).record(start.elapsed().as_secs_f64());
        }

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
        Ok(())
    }

    fn index_block(
        &self,
        _block: &grug::Block,
        _block_outcome: &grug::BlockOutcome,
        _ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: Arc<dyn QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        #[cfg(feature = "tracing")]
        tracing::info!("post_indexing: {block_height}");

        let block_to_index = ctx
            .get::<BlockToIndex>()
            .ok_or(grug_app::IndexerError::Database(
                "BlockToIndex not found".to_string(),
            ))?;

        let handle = self.runtime_handle.spawn({
            let context = self.context.clone();
            let block_to_index = block_to_index.clone();
            async move {
                // Transfer processing
                transfers::save_transfers(&context, block_height).await?;

                // Save accounts
                accounts::save_accounts(&context, &block_to_index, &*querier).await?;

                context.pubsub.publish_block_minted(block_height).await?;

                Ok::<(), grug_app::IndexerError>(())
            }
        });

        self.runtime_handle.block_on(async {
            handle
                .await
                .map_err(|e| grug_app::IndexerError::Database(e.to_string()))?
        })?;

        #[cfg(feature = "metrics")]
        histogram!(
            "indexer.dango.hooks.duration",
            "block_height" => block_height.to_string()
        )
        .record(start.elapsed().as_secs_f64());

        Ok(())
    }

    fn wait_for_finish(&self) -> grug_app::IndexerResult<()> {
        Ok(())
    }
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!("indexer.dango.hooks.duration", "Hook duration in seconds");
}
