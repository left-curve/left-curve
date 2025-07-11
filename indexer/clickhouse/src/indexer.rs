use {crate::context::Context, indexer_sql::indexer::RuntimeHandler};

#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

pub struct Indexer {
    context: Context,
    pub runtime_handler: RuntimeHandler,
}

impl Indexer {
    pub fn new(runtime_handler: RuntimeHandler, context: Context) -> Self {
        Self {
            context,
            runtime_handler,
        }
    }
}

impl grug_app::Indexer for Indexer {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "testing")]
        if self.context.is_mocked() {
            #[cfg(feature = "tracing")]
            tracing::info!("Clickhouse indexer is mocked");
            return Ok(());
        }
        #[cfg(feature = "tracing")]
        tracing::info!("Clickhouse indexer started");

        let handle = self.runtime_handler.spawn({
            let clickhouse_client = self.context.clickhouse_client().clone();
            async move {
                for migration in crate::migrations::create_tables::MIGRATIONS {
                    clickhouse_client
                        .query(migration)
                        .execute()
                        .await
                        .map_err(|e| {
                            grug_app::IndexerError::Database(format!(
                                "Failed to run migration: {e}"
                            ))
                        })?;
                }

                #[cfg(feature = "tracing")]
                tracing::info!("ran migrations successfully");

                Ok::<(), grug_app::IndexerError>(())
            }
        });

        self.runtime_handler
            .block_on(handle)
            .map_err(|e| grug_app::IndexerError::Database(e.to_string()))??;

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
        _block: &grug_types::Block,
        _block_outcome: &grug_types::BlockOutcome,
        _ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        Ok(())
    }

    fn post_indexing(
        &self,
        #[allow(unused_variables)] block_height: u64,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, "`post_indexing` work started");

        let clickhouse_client = self.context.clickhouse_client().clone();
        let querier = querier.clone();
        let ctx = ctx.clone();
        let context = self.context.clone();

        let handle = self.runtime_handler.spawn(async move {
            #[cfg(feature = "metrics")]
            let start = Instant::now();

            Self::store_candles(&clickhouse_client, querier, &ctx).await?;

            #[cfg(feature = "metrics")]
            histogram!(
                "indexer.clickhouse.post_indexing.duration",
                "block_height" => block_height.to_string()
            )
            .record(start.elapsed().as_secs_f64());

            if let Err(_err) = context.pubsub.publish_block_minted(block_height).await {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %_err, block_height, "Can't publish block minted in `post_indexing`");
                return Ok(());
            }

            #[cfg(feature = "tracing")]
            tracing::debug!(block_height, "`post_indexing` async work finished");

            Ok::<(), grug_app::IndexerError>(())
        });

        self.runtime_handler
            .block_on(handle)
            .map_err(|e| grug_app::IndexerError::Database(e.to_string()))??;

        Ok(())
    }
}
#[cfg(feature = "testing")]
impl Drop for Indexer {
    fn drop(&mut self) {
        let context = self.context.clone();

        self.runtime_handler.block_on(async move {
            if let Err(_err) = context.cleanup_test_database().await {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %_err, "Failed to cleanup test database");
            }
        })
    }
}
#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!(
        "indexer.clickhouse.post_indexing.duration",
        "Post indexing duration in seconds"
    );
}
