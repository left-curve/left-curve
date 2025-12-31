use {
    crate::context::Context,
    async_trait::async_trait,
    dango_indexer_sql_migration::{Migrator, MigratorTrait},
    grug::{BlockAndBlockOutcomeWithHttpDetails, Config, Json, Storage},
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
    pub context: Context,
}

impl Indexer {
    pub fn new(context: Context) -> Self {
        Self { context }
    }
}

#[async_trait]
impl grug_app::Indexer for Indexer {
    async fn last_indexed_block_height(&self) -> grug_app::IndexerResult<Option<u64>> {
        // TODO: Implement last_indexed_block_height
        Ok(None)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn start(&mut self, _storage: &dyn Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        Migrator::up(&self.context.db, None)
            .await
            .map_err(|e| grug_app::IndexerError::database(e.to_string()))?;

        #[cfg(feature = "metrics")]
        {
            transfers::init_metrics();
            accounts::init_metrics();
            init_metrics();

            histogram!("indexer.dango.start.duration").record(start.elapsed().as_secs_f64());
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn post_indexing(
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

        let context = self.context.clone();
        let block_to_index = block_to_index.clone();

        // Run transfer processing and account saving in parallel
        let ((), ()) =
            tokio::try_join!(transfers::save_transfers(&context, block_height), async {
                accounts::save_accounts(&context, &block_to_index, app_cfg)
                    .await
                    .inspect_err(|_| {
                        #[cfg(feature = "metrics")]
                        counter!("indexer.dango.hooks.accounts.errors.total").increment(1);
                    })
            })?;

        context
            .pubsub
            .publish(block_height)
            .await
            .map_err(|e| grug_app::IndexerError::hook(e.to_string()))
            .inspect_err(|_| {
                #[cfg(feature = "metrics")]
                counter!("indexer.dango.hooks.pubsub.errors.total").increment(1);
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
