use {
    crate::home_directory::HomeDirectory,
    clap::Parser,
    dango_app::ProposalPreparer,
    dango_genesis::build_rust_codes,
    dango_httpd::{graphql::build_schema, server::config_app},
    grug_app::{App, NullIndexer},
    grug_db_disk::DiskDb,
    grug_types::HashExt,
    grug_vm_hybrid::HybridVm,
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

        let codes = build_rust_codes();
        let vm = HybridVm::new(self.wasm_cache_capacity, [
            codes.account_factory.to_bytes().hash256(),
            codes.account_margin.to_bytes().hash256(),
            // codes.account_safe.to_bytes().hash256(),
            codes.account_spot.to_bytes().hash256(),
            codes.bank.to_bytes().hash256(),
            codes.dex.to_bytes().hash256(),
            codes.hyperlane.fee.to_bytes().hash256(),
            codes.hyperlane.ism.to_bytes().hash256(),
            codes.hyperlane.mailbox.to_bytes().hash256(),
            codes.hyperlane.merkle.to_bytes().hash256(),
            codes.hyperlane.va.to_bytes().hash256(),
            codes.lending.to_bytes().hash256(),
            codes.oracle.to_bytes().hash256(),
            codes.taxman.to_bytes().hash256(),
            codes.vesting.to_bytes().hash256(),
            codes.warp.to_bytes().hash256(),
        ]);

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
