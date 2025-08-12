#[cfg(feature = "testing")]
use clickhouse::test;
use {
    crate::{cache::CandleCache, entities::pair_price::PairPrice},
    clickhouse::Client,
    indexer_sql::pubsub::{self, PubSub},
    std::sync::Arc,
    tokio::sync::RwLock,
};

#[derive(Clone)]
pub struct Context {
    #[cfg(feature = "testing")]
    #[allow(dead_code)]
    mock: Option<Arc<test::Mock>>,
    #[cfg(feature = "testing")]
    #[allow(dead_code)]
    pub clickhouse_database: String,
    #[allow(dead_code)]
    clickhouse_client: Client,
    pub pubsub: Arc<dyn PubSub<u64> + Send + Sync>,
    pub candle_cache: Arc<RwLock<CandleCache>>,
}

impl Context {
    #[cfg(not(feature = "testing"))]
    pub fn new(url: String, database: String, user: String, password: String) -> Self {
        let clickhouse_client = Client::default()
            .with_url(&url)
            .with_user(&user)
            .with_password(&password)
            .with_database(&database);

        let pubsub: Arc<dyn PubSub<u64> + Send + Sync> = Arc::new(pubsub::MemoryPubSub::new(100));

        Self {
            clickhouse_client,
            pubsub,
            candle_cache: Default::default(),
        }
    }

    #[cfg(feature = "testing")]
    pub fn new(url: String, database: String, user: String, password: String) -> Self {
        let clickhouse_client = Client::default()
            .with_url(&url)
            .with_user(&user)
            .with_password(&password)
            .with_database(&database);

        #[cfg(feature = "tracing")]
        tracing::info!(
            "Clickhouse client created: {url}, user: {user}, password len: {}, database: {database}",
            password.len()
        );

        let pubsub: Arc<dyn PubSub<u64> + Send + Sync> = Arc::new(pubsub::MemoryPubSub::new(100));

        Self {
            mock: None,
            clickhouse_database: database,
            clickhouse_client,
            pubsub,
            candle_cache: Default::default(),
        }
    }

    pub async fn preload_candle_cache(&self) -> crate::error::Result<()> {
        let all_pairs = PairPrice::all_pairs(self.clickhouse_client()).await?;

        let mut candle_cache = self.candle_cache.write().await;

        candle_cache
            .preload_pairs(&all_pairs, self.clickhouse_client())
            .await
    }

    #[cfg(feature = "async-graphql")]
    pub async fn start_candle_cache(&self) -> crate::error::Result<()> {
        self.preload_candle_cache().await
    }

    #[cfg(feature = "testing")]
    pub fn with_mock(self) -> Self {
        let mock = test::Mock::new();

        Self {
            clickhouse_client: self.clickhouse_client.clone().with_mock(&mock),
            mock: Some(Arc::new(mock)),
            clickhouse_database: self.clickhouse_database.clone(),
            pubsub: self.pubsub.clone(),
            candle_cache: Default::default(),
        }
    }

    #[cfg(feature = "testing")]
    pub async fn with_test_database(self) -> crate::error::Result<Self> {
        let test_database = testing::generate_test_database_name();

        let create_db_sql = format!("CREATE DATABASE IF NOT EXISTS `{test_database}`",);
        self.clickhouse_client
            .query(&create_db_sql)
            .execute()
            .await?;

        #[cfg(feature = "tracing")]
        tracing::info!("Created test database: {test_database}");

        let clickhouse_client = self.clickhouse_client.with_database(&test_database);

        Ok(Self {
            mock: None,
            clickhouse_database: test_database,
            clickhouse_client,
            pubsub: self.pubsub,
            candle_cache: Default::default(),
        })
    }

    #[cfg(feature = "testing")]
    pub fn mock(&self) -> &test::Mock {
        self.mock.as_ref().unwrap()
    }

    pub fn clickhouse_client(&self) -> &Client {
        &self.clickhouse_client
    }

    pub fn is_mocked(&self) -> bool {
        #[cfg(feature = "testing")]
        return self.mock.is_some();

        #[cfg(not(feature = "testing"))]
        return false;
    }

    #[cfg(feature = "testing")]
    pub async fn cleanup_test_database(&self) -> Result<(), crate::error::IndexerError> {
        if self.is_mocked() {
            // No cleanup needed for mocked databases
            return Ok(());
        }

        let drop_sql = format!("DROP DATABASE IF EXISTS `{}`", self.clickhouse_database);

        #[cfg(feature = "tracing")]
        tracing::info!(
            database = self.clickhouse_database,
            "Cleaning up test database"
        );

        self.clickhouse_client.query(&drop_sql).execute().await?;

        #[cfg(feature = "tracing")]
        tracing::info!(
            database = self.clickhouse_database,
            "Cleaned up test database"
        );

        Ok(())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(feature = "testing")]
pub mod testing {
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DB_COUNTER: AtomicU64 = AtomicU64::new(1);

    /// Generates a unique database name for testing
    pub(crate) fn generate_test_database_name() -> String {
        let test_id = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        format!("dango_test_{test_id}_{timestamp}")
    }
}
