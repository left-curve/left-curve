use {
    crate::{
        config::{Config, GrugConfig, HttpdConfig, TendermintConfig},
        home_directory::HomeDirectory,
    },
    anyhow::anyhow,
    clap::Parser,
    config_parser::parse_config,
    dango_genesis::GenesisCodes,
    dango_proposal_preparer::ProposalPreparer,
    grug_app::{App, Db, Indexer, NaiveProposalPreparer, NullIndexer},
    grug_client::TendermintRpcClient,
    grug_db_disk_lite::DiskDbLite,
    grug_httpd::context::Context as HttpdContext,
    grug_types::{GIT_COMMIT, HashExt},
    grug_vm_hybrid::HybridVm,
    indexer_hooked::HookedIndexer,
    indexer_sql::indexer_path::IndexerPath,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    std::{sync::Arc, time},
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

        // Create the base app instance for HTTP server
        let app = App::new(
            db.clone(),
            vm.clone(),
            NaiveProposalPreparer,
            NullIndexer,
            cfg.grug.query_gas_limit,
        );

        let sql_indexer = indexer_sql::IndexerBuilder::default()
            .with_keep_blocks(cfg.indexer.keep_blocks)
            .with_database_url(&cfg.indexer.database.url)
            .with_database_max_connections(cfg.indexer.database.max_connections)
            .with_dir(app_dir.indexer_dir())
            .with_sqlx_pubsub()
            .build()
            .map_err(|err| anyhow!("failed to build indexer: {err:?}"))?;

        let indexer_path = sql_indexer.indexer_path.clone();
        let indexer_context = sql_indexer.context.clone();

        // Run ABCI server, optionally with indexer and httpd server.
        match (
            cfg.indexer.enabled,
            cfg.httpd.enabled,
            cfg.metrics_httpd.enabled,
        ) {
            (true, true, true) => {
                // Indexer, HTTP server, and metrics server all enabled
                let (hooked_indexer, _, dango_httpd_context) = self
                    .setup_indexer_stack(
                        sql_indexer,
                        indexer_context,
                        indexer_path,
                        Arc::new(app),
                        &cfg.tendermint.rpc_addr,
                    )
                    .await?;

                tokio::try_join!(
                    Self::run_dango_httpd_server(&cfg.httpd, dango_httpd_context,),
                    Self::run_metrics_httpd_server(&cfg.metrics_httpd, metrics_handler),
                    self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, hooked_indexer)
                )?;
            },
            (true, true, false) => {
                // Indexer and HTTP server enabled, metrics disabled
                let (hooked_indexer, _, dango_httpd_context) = self
                    .setup_indexer_stack(
                        sql_indexer,
                        indexer_context,
                        indexer_path,
                        Arc::new(app),
                        &cfg.tendermint.rpc_addr,
                    )
                    .await?;

                tokio::try_join!(
                    Self::run_dango_httpd_server(&cfg.httpd, dango_httpd_context,),
                    self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, hooked_indexer)
                )?;
            },
            (true, false, true) => {
                // Indexer and metrics enabled, HTTP server disabled
                let (hooked_indexer, ..) = self
                    .setup_indexer_stack(
                        sql_indexer,
                        indexer_context,
                        indexer_path,
                        Arc::new(app),
                        &cfg.tendermint.rpc_addr,
                    )
                    .await?;

                tokio::try_join!(
                    Self::run_metrics_httpd_server(&cfg.metrics_httpd, metrics_handler),
                    self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, hooked_indexer)
                )?;
            },
            (true, false, false) => {
                // Only indexer enabled
                let (hooked_indexer, ..) = self
                    .setup_indexer_stack(
                        sql_indexer,
                        indexer_context,
                        indexer_path,
                        Arc::new(app),
                        &cfg.tendermint.rpc_addr,
                    )
                    .await?;

                self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, hooked_indexer)
                    .await?;
            },
            (false, true, false) => {
                // No indexer, but HTTP server enabled (minimal mode), metrics disabled
                let httpd_context = HttpdContext::new(Arc::new(app));
                tokio::try_join!(
                    Self::run_minimal_httpd_server(&cfg.httpd, httpd_context),
                    self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, NullIndexer)
                )?;
            },
            (false, true, true) => {
                // No indexer, but HTTP server enabled (minimal mode), metrics enabled
                let httpd_context = HttpdContext::new(Arc::new(app));
                tokio::try_join!(
                    Self::run_minimal_httpd_server(&cfg.httpd, httpd_context),
                    self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, NullIndexer),
                    Self::run_metrics_httpd_server(&cfg.metrics_httpd, metrics_handler)
                )?;
            },
            (false, false, _) => {
                // No indexer, no HTTP server
                self.run_with_indexer(cfg.grug, cfg.tendermint, db, vm, NullIndexer)
                    .await?;
            },
        }

        Ok(())
    }

    /// Setup the hooked indexer with both SQL and Dango indexers, and prepare contexts for HTTP servers
    async fn setup_indexer_stack(
        &self,
        sql_indexer: indexer_sql::NonBlockingIndexer,
        indexer_context: indexer_sql::context::Context,
        indexer_path: IndexerPath,
        app: Arc<App<DiskDbLite, HybridVm, NaiveProposalPreparer, NullIndexer>>,
        tendermint_rpc_addr: &str,
    ) -> anyhow::Result<(
        HookedIndexer,
        indexer_httpd::context::Context,
        dango_httpd::context::Context,
    )> {
        let mut hooked_indexer = HookedIndexer::new();

        // Create a separate context for dango indexer (shares DB but has independent pubsub)
        let dango_context: dango_indexer_sql::context::Context = sql_indexer
            .context
            .with_separate_pubsub()
            .await
            .map_err(|e| anyhow!("Failed to create separate context for dango indexer: {}", e))?
            .into();

        let dango_indexer = dango_indexer_sql::indexer::Indexer {
            runtime_handle: indexer_sql::indexer::RuntimeHandler::from_handle(
                sql_indexer.handle.handle().clone(),
            ),
            context: dango_context.clone(),
        };

        hooked_indexer.add_indexer(sql_indexer)?;
        hooked_indexer.add_indexer(dango_indexer)?;

        let indexer_httpd_context = indexer_httpd::context::Context::new(
            indexer_context,
            app.clone(),
            Arc::new(TendermintRpcClient::new(tendermint_rpc_addr)?),
            indexer_path,
        );

        let dango_httpd_context =
            dango_httpd::context::Context::new(indexer_httpd_context.clone(), dango_context);

        Ok((hooked_indexer, indexer_httpd_context, dango_httpd_context))
    }

    /// Run the minimal HTTP server (without indexer features)
    async fn run_minimal_httpd_server(
        cfg: &HttpdConfig,
        context: HttpdContext,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting minimal HTTP server at {}:{}", &cfg.ip, cfg.port);

        grug_httpd::server::run_server(
            &cfg.ip,
            cfg.port,
            cfg.cors_allowed_origin.clone(),
            context,
            grug_httpd::server::config_app,
            grug_httpd::graphql::build_schema,
        )
        .await
        .map_err(|err| {
            tracing::error!("Failed to run minimal HTTP server: {err:?}");
            err.into()
        })
    }

    /// Run the full-featured HTTP server (with indexer features)
    async fn run_dango_httpd_server(
        cfg: &HttpdConfig,
        dango_httpd_context: dango_httpd::context::Context,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Starting full-featured HTTP server at {}:{}",
            &cfg.ip,
            cfg.port
        );

        dango_httpd::server::run_server(
            &cfg.ip,
            cfg.port,
            cfg.cors_allowed_origin.clone(),
            dango_httpd_context,
        )
        .await
        .map_err(|err| {
            tracing::error!("Failed to run full-featured HTTP server: {err:?}");
            err.into()
        })
    }

    /// Run the metrics HTTP server
    async fn run_metrics_httpd_server(
        cfg: &HttpdConfig,
        metrics_handler: PrometheusHandle,
    ) -> anyhow::Result<()> {
        indexer_httpd::server::run_metrics_server(&cfg.ip, cfg.port, metrics_handler)
            .await
            .map_err(|err| {
                tracing::error!("Failed to run metrics HTTP server: {err:?}");
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
