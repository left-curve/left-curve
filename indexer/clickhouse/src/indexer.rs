use {clickhouse::Client, indexer_sql::indexer::RuntimeHandler};

#[cfg(feature = "testing")]
use clickhouse::test;

#[cfg(feature = "metrics")]
use {
    metrics::{describe_histogram, histogram},
    std::time::Instant,
};

pub struct Indexer {
    #[cfg(feature = "testing")]
    #[allow(dead_code)]
    mock: Option<test::Mock>,
    #[allow(dead_code)]
    clickhouse_client: Client,
    pub runtime_handler: RuntimeHandler,
}

impl Indexer {
    #[cfg(not(feature = "testing"))]
    pub fn new(
        runtime_handler: RuntimeHandler,
        url: String,
        database: String,
        user: String,
        password: String,
    ) -> grug_app::IndexerResult<Self> {
        let clickhouse_client = Client::default()
            .with_url(&url)
            .with_user(&user)
            .with_password(&password)
            .with_database(&database);

        init_metrics();

        Ok(Self {
            clickhouse_client,
            runtime_handler,
        })
    }

    #[cfg(feature = "testing")]
    pub fn new(
        runtime_handler: RuntimeHandler,
        url: String,
        database: String,
        user: String,
        password: String,
    ) -> grug_app::IndexerResult<Self> {
        let clickhouse_client = Client::default()
            .with_url(&url)
            .with_user(&user)
            .with_password(&password)
            .with_database(&database);

        init_metrics();

        Ok(Self {
            mock: None,
            clickhouse_client,
            runtime_handler,
        })
    }

    #[cfg(feature = "testing")]
    pub fn mock(self) -> Self {
        let mock = test::Mock::new();

        Self {
            clickhouse_client: self.clickhouse_client.with_mock(&mock),
            mock: Some(mock),
            runtime_handler: self.runtime_handler,
        }
    }
}

impl Indexer {
    pub fn clickhouse_client(&self) -> &Client {
        &self.clickhouse_client
    }
}

impl grug_app::Indexer for Indexer {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        // TODO: run migrations
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
        block_height: u64,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, "`post_indexing` work started");

        let clickhouse_client = self.clickhouse_client.clone();
        let querier = querier.clone();
        let mut ctx = ctx.clone();

        let handle = self.runtime_handler.spawn(async move {
            #[cfg(feature = "metrics")]
            let start = Instant::now();

            Self::store_candles(&clickhouse_client, querier, &mut ctx).await?;

            #[cfg(feature = "metrics")]
            histogram!(
                "indexer.clickhouse.post_indexing.duration",
                "block_height" => block_height.to_string()
            )
            .record(start.elapsed().as_secs_f64());

            Ok::<(), grug_app::IndexerError>(())
        });

        self.runtime_handler
            .block_on(handle)
            .map_err(|e| grug_app::IndexerError::Database(e.to_string()))??;

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, "`post_indexing` async work finished");

        Ok(())
    }
}

#[cfg(feature = "metrics")]
pub fn init_metrics() {
    describe_histogram!(
        "indexer.clickhouse.post_indexing.duration",
        "Post indexing duration in seconds"
    );
}
