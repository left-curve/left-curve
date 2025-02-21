use {
    crate::{home_directory::HomeDirectory, start::vm},
    clap::Parser,
    dango_app::ProposalPreparer,
    dango_httpd::{graphql::build_schema, server::config_app},
    grug_app::{App, NullIndexer},
    grug_db_disk::DiskDb,
    indexer_httpd::context::Context,
    indexer_sql::non_blocking_indexer,
    std::sync::Arc,
};

#[derive(Parser)]
pub struct HttpdCmd {
    /// Capacity of the wasm module cache; zero means do not use a cache
    #[arg(long, default_value = "1000")]
    wasm_cache_capacity: usize,

    /// Gas limit when serving query requests
    #[arg(long, default_value_t = u64::MAX)]
    query_gas_limit: u64,

    /// The indexer database url
    #[arg(long, default_value = "postgres://localhost")]
    indexer_database_url: String,
}

impl HttpdCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        let db = DiskDb::open(app_dir.data_dir())?;

        let vm = vm(self.wasm_cache_capacity);

        let context = non_blocking_indexer::IndexerBuilder::default()
            .with_database_url(&self.indexer_database_url)
            .with_sqlx_pubsub()
            .build_context()
            .expect("Can't create indexer context");

        let app = App::new(
            db,
            vm,
            ProposalPreparer::new(),
            NullIndexer,
            self.query_gas_limit,
        );

        let httpd_context = Context::new(context, Arc::new(app));

        indexer_httpd::server::run_server(None, None, httpd_context, config_app, build_schema)
            .await?;

        Ok(())
    }
}
