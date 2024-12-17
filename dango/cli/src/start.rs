use {
    anyhow::anyhow,
    clap::Parser,
    dango_app::ProposalPreparer,
    grug_app::{App, AppError, Db, HomeDirectory, Indexer, NullIndexer},
    grug_db_disk::DiskDb,
    grug_vm_wasm::WasmVm,
    indexer_sql::non_blocking_indexer,
    std::time,
    tower::ServiceBuilder,
    tower_abci::v038::{split, Server},
};

#[derive(Parser)]
pub struct StartCmd {
    /// Tendermint ABCI listening address
    #[arg(long, default_value = "127.0.0.1:26658")]
    abci_addr: String,

    /// Capacity of the wasm module cache; zero means do not use a cache
    #[arg(long, default_value = "1000")]
    wasm_cache_capacity: usize,

    /// Gas limit when serving query requests
    #[arg(long, default_value_t = u64::MAX)]
    query_gas_limit: u64,

    /// Enable the internal indexer
    #[arg(long, default_value = "false")]
    indexer_enabled: bool,

    /// Enable the indexer httpd
    #[arg(long, default_value = "false")]
    indexer_httpd_enabled: bool,

    /// Enable the internal indexer
    #[arg(long, default_value = "false")]
    indexer_keep_blocks: bool,

    /// The indexer database url
    #[arg(long, default_value = "postgres://localhost")]
    indexer_database_url: String,
}

impl StartCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        if self.indexer_enabled {
            let indexer = non_blocking_indexer::IndexerBuilder::default()
                .with_keep_blocks(self.indexer_keep_blocks)
                .with_database_url(&self.indexer_database_url)
                .with_dir(app_dir.indexer_dir())
                .build()
                .expect("Can't create indexer");
            self.run_with_indexer(app_dir, indexer).await
        } else {
            self.run_with_indexer(app_dir, NullIndexer).await
        }
    }

    async fn run_with_indexer<ID>(
        self,
        app_dir: HomeDirectory,
        mut indexer: ID,
    ) -> anyhow::Result<()>
    where
        ID: Indexer + Send + 'static,
        AppError: From<ID::Error>,
    {
        let db = DiskDb::open(app_dir.data_dir())?;
        let vm = WasmVm::new(self.wasm_cache_capacity);

        indexer
            .start(&db.state_storage(None)?)
            .expect("Can't start indexer");

        let app = App::new(
            db,
            vm,
            ProposalPreparer::new(),
            indexer,
            self.query_gas_limit,
        );

        let (consensus, mempool, snapshot, info) = split::service(app, 1);

        let mempool = ServiceBuilder::new()
            .load_shed()
            .buffer(100)
            .service(mempool);

        let info = ServiceBuilder::new()
            .load_shed()
            .buffer(100)
            .rate_limit(50, time::Duration::from_secs(1))
            .service(info);

        Server::builder()
            .consensus(consensus)
            .snapshot(snapshot)
            .mempool(mempool)
            .info(info)
            .finish()
            .unwrap() // this fails if one of consensus|snapshot|mempool|info is None
            .listen_tcp(self.abci_addr)
            .await
            .map_err(|err| anyhow!("failed to start tower ABCI server: {err}"))
    }
}
