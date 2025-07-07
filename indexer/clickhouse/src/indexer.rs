pub struct Indexer {
    client: clickhouse::Client,
}

impl Indexer {
    pub fn new(url: String, database: String, user: String, password: String) -> Self {
        let client = clickhouse::Client::default()
            .with_url(url)
            .with_user(user)
            .with_password(password)
            .with_database(database);

        Self { client }
    }
}

impl grug_app::Indexer for Indexer {
    fn start(&mut self, _storage: &dyn grug_types::Storage) -> grug_app::IndexerResult<()> {
        todo!("run migrations")
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
        _block_height: u64,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        self.store_candles(querier, ctx)
    }
}
