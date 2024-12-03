use {
    anyhow::anyhow,
    clap::Parser,
    dango_app::ProposalPreparer,
    grug_app::{App, AppError, Indexer, NullIndexer},
    grug_db_disk::DiskDb,
    grug_vm_wasm::WasmVm,
    indexer_sql::non_blocking_indexer,
    std::{path::PathBuf, time},
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

    /// Gas limit when serving query requests [default: u64::MAX]
    #[arg(long)]
    query_gas_limit: Option<u64>,

    /// Enable the internal indexer
    #[arg(long, default_value = "false")]
    indexer_enabled: bool,

    /// The indexer database url
    #[arg(long, default_value = "postgres://localhost")]
    indexer_database_url: String,
}

impl StartCmd {
    pub async fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        if self.indexer_enabled {
            let mut indexer = non_blocking_indexer::IndexerBuilder::default()
                .with_database_url(&self.indexer_database_url)
                .build()
                .expect("Can't create indexer");
            indexer.start().expect("Can't start indexer");
            self.run_with_indexer(data_dir, indexer).await
        } else {
            self.run_with_indexer(data_dir, NullIndexer).await
        }
    }

    async fn run_with_indexer<ID>(self, data_dir: PathBuf, indexer: ID) -> anyhow::Result<()>
    where
        ID: Indexer + Send + Clone + 'static,
        AppError: From<ID::Error>,
    {
        let db = DiskDb::open(data_dir)?;
        let vm = WasmVm::new(self.wasm_cache_capacity);
        let app = App::new(
            db,
            vm,
            ProposalPreparer::new(),
            indexer,
            self.query_gas_limit.unwrap_or(u64::MAX),
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
