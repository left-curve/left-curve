use {
    crate::{context::Context, error::Error},
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    grug::{BlockAndBlockOutcomeWithHttpDetails, Config, Json, Storage},
    indexer_sql::indexer::RuntimeHandler,
};
#[cfg(feature = "metrics")]
use {
    metrics::counter,
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

mod accounts;
mod transfers;

pub struct Indexer {
    runtime_handler: RuntimeHandler,
    pub context: Context,
}

impl Indexer {
    pub fn new(runtime_handler: RuntimeHandler, context: Context) -> Self {
        Self {
            runtime_handler,
            context,
        }
    }
}

impl grug_app::Indexer for Indexer {
    fn last_indexed_block_height(&self) -> grug_app::IndexerResult<Option<u64>> {
        // TODO: Implement last_indexed_block_height for indexer, looking at the last
        // cached block from SQL
        Ok(None)
    }

    fn start(&mut self, _storage: &dyn Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        self.runtime_handler.block_on(async {
            Migrator::up(&self.context.db, None)
                .await
                .map_err(|e| grug_app::IndexerError::database(e.to_string()))?;

            Ok::<(), grug_app::IndexerError>(())
        })?;

        #[cfg(feature = "metrics")]
        {
            transfers::init_metrics();
            accounts::init_metrics();
            init_metrics();

            histogram!("indexer.dango.start.duration").record(start.elapsed().as_secs_f64());
        }

        Ok(())
    }

    fn post_indexing(
        &self,
        block_height: u64,
        _cfg: Config,
        app_cfg: Json,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let block_to_index = ctx.get::<BlockAndBlockOutcomeWithHttpDetails>().ok_or(
            grug_app::IndexerError::hook(
                "BlockAndBlockOutcomeWithHttpDetails not found".to_string(),
            ),
        )?;

        self.runtime_handler.block_on({
            let context = self.context.clone();
            let block_to_index = block_to_index.clone();
            async move {
                // Transfer processing
                transfers::save_transfers(&context, block_height).await?;

                // Save accounts
                accounts::save_accounts(&context, &block_to_index, app_cfg)
                    .await
                    .inspect_err(|_| {
                        #[cfg(feature = "metrics")]
                        counter!("indexer.dango.hooks.accounts.errors.total").increment(1);
                    })?;

                context
                    .pubsub
                    .publish(block_height)
                    .await
                    .inspect_err(|_| {
                        #[cfg(feature = "metrics")]
                        counter!("indexer.dango.hooks.pubsub.errors.total").increment(1);
                    })?;

                Ok::<(), Error>(())
            }
        })?;

        #[cfg(feature = "metrics")]
        histogram!("indexer.dango.hooks.duration").record(start.elapsed().as_secs_f64());

        Ok(())
    }
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!("indexer.dango.hooks.duration", "Hook duration in seconds");
}
