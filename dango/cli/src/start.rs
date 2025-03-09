use {
    crate::{
        config::{parse_config, Config},
        home_directory::HomeDirectory,
    },
    anyhow::anyhow,
    clap::Parser,
    dango_app::ProposalPreparer,
    dango_genesis::build_rust_codes,
    dango_httpd::{graphql::build_schema, server::config_app},
    grug_app::{App, AppError, Db, Indexer, NullIndexer},
    grug_db_disk::DiskDb,
    grug_types::HashExt,
    grug_vm_hybrid::HybridVm,
    indexer_httpd::context::Context,
    indexer_sql::non_blocking_indexer,
    std::{fmt::Debug, path::PathBuf, sync::Arc, time},
    tokio::signal::unix::{signal, SignalKind},
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

    /// Optional path to the configuration file
    #[arg(long)]
    config_file: Option<PathBuf>,
}

impl StartCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        // Parse the config file.
        let cfg = parse_config(app_dir.config_file())?;

        // Open disk DB.
        let db = DiskDb::open(app_dir.data_dir())?;

        // Create hybird VM.
        let codes = build_rust_codes();
        let vm = HybridVm::new(self.wasm_cache_capacity, [
            codes.account_factory.to_bytes().hash256(),
            codes.account_margin.to_bytes().hash256(),
            codes.account_multi.to_bytes().hash256(),
            codes.account_spot.to_bytes().hash256(),
            codes.bank.to_bytes().hash256(),
            codes.dex.to_bytes().hash256(),
            codes.hyperlane.ism.to_bytes().hash256(),
            codes.hyperlane.mailbox.to_bytes().hash256(),
            codes.hyperlane.va.to_bytes().hash256(),
            codes.lending.to_bytes().hash256(),
            codes.oracle.to_bytes().hash256(),
            codes.taxman.to_bytes().hash256(),
            codes.vesting.to_bytes().hash256(),
            codes.warp.to_bytes().hash256(),
        ]);

        // Run ABCI server, optionally with indexer and httpd server.
        if cfg.indexer.enabled {
            let indexer = non_blocking_indexer::IndexerBuilder::default()
                .with_keep_blocks(cfg.indexer.keep_blocks)
                .with_database_url(&cfg.indexer.postgres_url)
                .with_dir(app_dir.indexer_dir())
                .with_sqlx_pubsub()
                .build()
                .map_err(|err| anyhow!("failed to build indexer: {err:?}"))?;

            let app = App::new(
                db.clone(),
                vm.clone(),
                ProposalPreparer::new(),
                NullIndexer,
                self.query_gas_limit,
            );

            if cfg.indexer.httpd.enabled {
                let httpd_context = Context::new(
                    indexer.context.clone(),
                    Arc::new(app),
                    cfg.tendermint.rpc_addr.clone(),
                );

                // NOTE: If the httpd was heavily used, it would be better to
                // run it in a separate tokio runtime.
                tokio::try_join!(
                    Self::run_httpd_server(cfg, httpd_context),
                    self.run_with_indexer(db, vm, indexer)
                )?;

                Ok(())
            } else {
                self.run_with_indexer(db, vm, indexer).await
            }
        } else {
            self.run_with_indexer(db, vm, NullIndexer).await
        }
    }

    /// Run the HTTP server
    async fn run_httpd_server(cfg: Config, context: Context) -> anyhow::Result<()> {
        indexer_httpd::server::run_server(
            &cfg.indexer.httpd.ip,
            cfg.indexer.httpd.port,
            cfg.indexer.httpd.cors_allowed_origin,
            context,
            config_app,
            build_schema,
        )
        .await
        .map_err(|err| {
            tracing::error!("Failed to run HTTP server: {err:?}");
            err.into()
        })
    }

    async fn run_with_indexer<ID>(
        self,
        db: DiskDb,
        vm: HybridVm,
        mut indexer: ID,
    ) -> anyhow::Result<()>
    where
        ID: Indexer + Send + 'static,
        ID::Error: Debug,
        AppError: From<ID::Error>,
    {
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

        let abci_server = Server::builder()
            .consensus(consensus)
            .snapshot(snapshot)
            .mempool(mempool)
            .info(info)
            .finish()
            .unwrap(); // this fails if one of consensus|snapshot|mempool|info is None

        // Listen for SIGINT and SIGTERM signals.
        // SIGINT is received when user presses Ctrl-C.
        // SIGTERM is received when user does `systemctl stop`.
        let mut sigint = signal(SignalKind::interrupt())?;
        let mut sigterm = signal(SignalKind::terminate())?;

        tokio::select! {
            result = async { abci_server.listen_tcp(self.abci_addr).await } => {
                result.map_err(|err| anyhow!("failed to start ABCI server: {err:?}"))
            },
            _ = sigint.recv() => {
                tracing::info!("Received SIGINT, shutting down");
                Ok(())
            },
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM, shutting down");
                Ok(())
            },
        }
    }
}
