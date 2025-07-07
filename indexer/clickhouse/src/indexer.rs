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
        todo!()
    }

    fn shutdown(&mut self) -> grug_app::IndexerResult<()> {
        todo!()
    }

    fn pre_indexing(
        &self,
        block_height: u64,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        todo!()
    }

    fn index_block(
        &self,
        block: &grug_types::Block,
        block_outcome: &grug_types::BlockOutcome,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        todo!()
    }

    fn post_indexing(
        &self,
        block_height: u64,
        querier: std::sync::Arc<dyn grug_app::QuerierProvider>,
        ctx: &mut grug_app::IndexerContext,
    ) -> grug_app::IndexerResult<()> {
        todo!()
    }

    fn wait_for_finish(&self) -> grug_app::IndexerResult<()> {
        todo!()
    }
}
