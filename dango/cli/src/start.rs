use {
    crate::{
        config::{Config, GrugConfig, HttpdConfig, TendermintConfig},
        home_directory::HomeDirectory,
    },
    anyhow::anyhow,
    clap::Parser,
    config_parser::parse_config,
    dango_genesis::GenesisCodes,
    dango_httpd::{graphql::build_schema, server::config_app},
    dango_proposal_preparer::ProposalPreparer,
    grug_app::{App, AppError, Db, Indexer, NaiveProposalPreparer, NullIndexer},
    grug_client::TendermintRpcClient,
    grug_db_disk_lite::DiskDbLite,
    grug_types::{GIT_COMMIT, HashExt},
    grug_vm_hybrid::HybridVm,
    indexer_httpd::context::Context,
    indexer_sql::non_blocking_indexer,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    std::{fmt::Debug, sync::Arc, time},
    tokio::signal::unix::{SignalKind, signal},
    tower::ServiceBuilder,
    tower_abci::v038::{Server, split},
};

#[derive(Parser)]
pub struct StartCmd;

impl StartCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        tracing::info!("Using git commit: {GIT_COMMIT}");
        // Initialize metrics handler.
        // This should be done as soon as possible to capture all events.
        let metrics_handler = PrometheusBuilder::new().install_recorder()?;

        // Parse the config file.
        let cfg: Config = parse_config(app_dir.config_file())?;

        // Open disk DB.
        let db = DiskDbLite::open(app_dir.data_dir())?;

        // Create Rust VM contract codes.
        let codes = HybridVm::genesis_codes();

        // Create hybird VM.
        let vm = HybridVm::new(cfg.grug.wasm_cache_capacity, [
            codes.account_factory.to_bytes().hash256(),
            codes.account_margin.to_bytes().hash256(),
            codes.account_multi.to_bytes().hash256(),
            codes.account_spot.to_bytes().hash256(),
            codes.bank.to_bytes().hash256(),
            codes.dex.to_bytes().hash256(),
            codes.gateway.to_bytes().hash256(),
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
                .with_database_url(&cfg.indexer.database.url)
                .with_database_max_connections(cfg.indexer.database.max_connections)
                .with_dir(app_dir.indexer_dir())
                .with_sqlx_pubsub()
                .with_hooks(dango_indexer_sql::hooks::Hooks)
                .build()
                .map_err(|err| anyhow!("failed to build indexer: {err:?}"))?;

            let indexer_path = indexer.indexer_path.clone();
            let indexer_context = indexer.context.clone();

            if cfg.indexer.httpd.enabled {
                // This app instance allows the httpd daemon to interact with the chain
                // but it doesn't need to have an indexer at all.
                let app = App::new(
                    db.clone(),
                    vm.clone(),
                    NaiveProposalPreparer,
                    NullIndexer,
                    cfg.grug.query_gas_limit,
                );

                let httpd_context = Context::new(
                    indexer_context,
                    Arc::new(app),
                    Arc::new(TendermintRpcClient::new(&cfg.tendermint.rpc_addr)?),
                    indexer_path,
                );

                // NOTE: If the httpd was heavily used, it would be better to
                // run it in a separate tokio runtime.
                tokio::try_join!(
                    Self::run_httpd_server(&cfg.indexer.httpd, httpd_context),
                    Self::run_metrics_httpd_server(&cfg.indexer.metrics_httpd, metrics_handler),
                    self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, indexer)
                )?;

                Ok(())
            } else {
                self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, indexer)
                    .await
            }
        } else {
            self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, NullIndexer)
                .await
        }
    }

    /// Run the indexer HTTP server
    async fn run_httpd_server(cfg: &HttpdConfig, context: Context) -> anyhow::Result<()> {
        if !cfg.enabled {
            tracing::info!("HTTP server is disabled in the configuration.");
            return Ok(());
        }

        indexer_httpd::server::run_server(
            &cfg.ip,
            cfg.port,
            cfg.cors_allowed_origin.clone(),
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

    /// Run the metrics HTTP server
    async fn run_metrics_httpd_server(
        cfg: &HttpdConfig,
        metrics_handler: PrometheusHandle,
    ) -> anyhow::Result<()> {
        if !cfg.enabled {
            tracing::info!("Metrics HTTP server is disabled in the configuration.");
            return Ok(());
        }

        indexer_httpd::server::run_metrics_server(&cfg.ip, cfg.port, metrics_handler)
            .await
            .map_err(|err| {
                tracing::error!("Failed to run HTTP server: {err:?}");
                err.into()
            })
    }

    async fn run_with_indexer<ID>(
        self,
        grug_cfg: GrugConfig,
        tendermint_cfg: TendermintConfig,
        db: DiskDbLite,
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
            grug_cfg.query_gas_limit,
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
            result = async { abci_server.listen_tcp(tendermint_cfg.abci_addr).await } => {
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
