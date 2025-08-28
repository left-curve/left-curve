use {
    crate::{context::Context, error::IndexerError},
    dango_types::DangoQuerier,
    futures::try_join,
    grug_app::Indexer as IndexerTrait,
    indexer_sql::indexer::RuntimeHandler,
};

#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

pub mod candles;
pub mod trades;

pub struct Indexer {
    pub context: Context,
    pub runtime_handler: RuntimeHandler,
    indexing: bool,
}

impl Indexer {
    pub fn new(runtime_handler: RuntimeHandler, context: Context) -> Self {
        Self {
            context,
            runtime_handler,
            indexing: false,
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
                for migration in crate::migrations::candle_builder::migrations()
                    .iter()
                    .chain(crate::migrations::trade::Migration::migrations().iter())
                {
                    clickhouse_client
                        .query(migration)
                        .execute()
                        .await
                        .map_err(|e| {
                            grug_app::IndexerError::Database(format!(
                                "Failed to run migration: {e}"
                            ))
                        })?;

                    #[cfg(feature = "tracing")]
                    tracing::debug!("ran migration: {migration}");
                }

                #[cfg(feature = "tracing")]
                tracing::info!("ran migrations successfully");

                Ok::<(), grug_app::IndexerError>(())
            }
        });

        self.runtime_handler
            .block_on(handle)
            .map_err(|e| grug_app::IndexerError::Hook(e.to_string()))??;

        self.indexing = true;

        Ok(())
    }

    fn wait_for_finish(&self) -> grug_app::IndexerResult<()> {
        if !self.indexing {
            return Ok(());
        }

        self.runtime_handler.block_on(async move {
            let candle_generator = candles::generator::CandleGenerator::new(self.context.clone());

            if let Err(_err) = candle_generator.save_all_candles().await {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %_err, "Failed to save candles");
            }

            Ok::<(), grug_app::IndexerError>(())
        })
    }

    fn shutdown(&mut self) -> grug_app::IndexerResult<()> {
        // Avoid running this twice when called manually and from `Drop`
        if !self.indexing {
            return Ok(());
        }

        self.wait_for_finish()?;

        self.indexing = false;

        #[cfg(feature = "testing")]
        {
            let context = self.context.clone();
            self.runtime_handler.block_on(async move {
                if let Err(_err) = context.cleanup_test_database().await {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(err = %_err, "Failed to cleanup test database");
                }
            });
        }

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
        if !self.indexing {
            return Err(grug_app::IndexerError::NotRunning);
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, "`post_indexing` work started");

        let querier = querier.clone();
        let ctx = ctx.clone();
        let context = self.context.clone();

        let handle = self.runtime_handler.spawn(async move {
            #[cfg(feature = "metrics")]
            let start = Instant::now();

            let dex_addr = querier.as_ref().query_dex()?;

            try_join!(
                Self::store_candles(&dex_addr, &ctx, &context),
                Self::store_trades(&dex_addr, &ctx, &context)
            )?;

            #[cfg(feature = "metrics")]
            histogram!(
                "indexer.clickhouse.post_indexing.duration"
            )
            .record(start.elapsed().as_secs_f64());

            if let Err(_err) = context.pubsub.publish(block_height).await {
                #[cfg(feature = "tracing")]
                tracing::error!(err = %_err, block_height, "Can't publish block minted in `post_indexing`");
                return Ok(());
            }

            #[cfg(feature = "tracing")]
            tracing::debug!(block_height, "`post_indexing` async work finished");

            Ok::<(), IndexerError>(())
        });

        self.runtime_handler
            .block_on(handle)
            .map_err(|e| grug_app::IndexerError::Hook(e.to_string()))??;

        Ok(())
    }
}

impl Drop for Indexer {
    fn drop(&mut self) {
        self.shutdown().expect("can't shutdown indexer");
    }
}
#[cfg(feature = "metrics")]
pub fn init_metrics() {
    use metrics::{describe_counter, describe_gauge};

    describe_histogram!(
        "indexer.clickhouse.post_indexing.duration",
        "Post indexing duration in seconds"
    );

    describe_counter!(
        "indexer.clickhouse.candles.cache.hits",
        "Number of candle cache hits"
    );

    describe_counter!(
        "indexer.clickhouse.candles.cache.misses",
        "Number of candle cache misses"
    );

    describe_histogram!(
        "indexer.clickhouse.candles.cache.lookup.duration.seconds",
        "Time spent on cache lookups"
    );

    describe_gauge!(
        "indexer.clickhouse.candles.cache.size.entries",
        "Current number of keys in cache"
    );

    describe_gauge!(
        "indexer.clickhouse.candles.cache.size.candles",
        "Total number of candles in cache"
    );

    describe_gauge!(
        "indexer.clickhouse.pair_prices.cache.size.entries",
        "Current number of keys in cache"
    );

    describe_gauge!(
        "indexer.clickhouse.pair_prices.cache.size.pair_prices",
        "Total number of pair_prices in cache"
    );

    describe_counter!(
        "indexer.clickhouse.order_filled_events.total",
        "Total order filled events processed"
    );

    describe_counter!(
        "indexer.clickhouse.pair_prices.processed.total",
        "Total pair prices processed"
    );

    describe_counter!(
        "indexer.clickhouse.candles.stored.total",
        "Total candles stored"
    );

    describe_counter!(
        "indexer.clickhouse.trades.processed.total",
        "Total trades processed"
    );

    describe_counter!(
        "indexer.clickhouse.synthetic_prices.total",
        "Total synthetic pair prices injected"
    );

    describe_counter!(
        "indexer.clickhouse.volume_overflow.total",
        "Total volume calculation overflows"
    );

    describe_counter!(
        "indexer.clickhouse.mv_wait_cycles.total",
        "Total materialized view wait cycles"
    );
}
