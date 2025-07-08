use {clickhouse::Client, indexer_sql::pubsub::PubSub, std::sync::Arc};

#[cfg(feature = "testing")]
use clickhouse::test;

#[derive(Clone)]
pub struct Context {
    #[cfg(feature = "testing")]
    #[allow(dead_code)]
    mock: Option<Arc<test::Mock>>,
    #[allow(dead_code)]
    clickhouse_client: Client,
    pub pubsub: Arc<dyn PubSub + Send + Sync>,
}

impl Context {
    #[cfg(not(feature = "testing"))]
    pub fn new(
        indexer_context: indexer_sql::context::Context,
        url: String,
        database: String,
        user: String,
        password: String,
    ) -> Self {
        let clickhouse_client = Client::default()
            .with_url(&url)
            .with_user(&user)
            .with_password(&password)
            .with_database(&database);

        Self {
            clickhouse_client,
            pubsub: indexer_context.pubsub,
        }
    }

    #[cfg(feature = "testing")]
    pub fn new(
        indexer_context: indexer_sql::context::Context,
        url: String,
        database: String,
        user: String,
        password: String,
    ) -> Self {
        let clickhouse_client = Client::default()
            .with_url(&url)
            .with_user(&user)
            .with_password(&password)
            .with_database(&database);

        Self {
            mock: None,
            clickhouse_client,
            pubsub: indexer_context.pubsub,
        }
    }

    #[cfg(feature = "testing")]
    pub fn with_mock(self) -> Self {
        let mock = test::Mock::new();

        Self {
            clickhouse_client: self.clickhouse_client.with_mock(&mock),
            mock: Some(Arc::new(mock)),
            pubsub: self.pubsub,
        }
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
}
