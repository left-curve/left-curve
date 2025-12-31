use {
    crate::{
        config::{Config, GrugConfig, HttpdConfig, PythLazerConfig, TendermintConfig},
        home_directory::HomeDirectory,
        telemetry,
    },
    anyhow::anyhow,
    clap::Parser,
    config_parser::parse_config,
    dango_genesis::GenesisCodes,
    dango_proposal_preparer::ProposalPreparer,
    grug_app::{App, Db, Indexer, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_client::TendermintRpcClient,
    grug_db_disk::DiskDb,
    grug_httpd::context::Context as HttpdContext,
    grug_types::GIT_COMMIT,
    grug_vm_rust::RustVm,
    indexer_hooked::HookedIndexer,
    metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle},
    std::sync::Arc,
    tokio::signal::unix::{SignalKind, signal},
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

        tracing::info!("Metrics handler initialized");

        // Parse the config file.
        let cfg: Config = parse_config(app_dir.config_file())?;

        // Emit startup logs now that the subscriber is initialized.
        if cfg.sentry.enabled {
            tracing::info!("Sentry initialized");
        } else {
            tracing::info!("Sentry is disabled");
        }
        if cfg.trace.enabled {
            tracing::info!(endpoint = %cfg.trace.endpoint, protocol = ?cfg.trace.protocol, "OpenTelemetry OTLP exporter initialized");
        } else {
            tracing::info!("OpenTelemetry OTLP exporter is disabled");
        }

        // Open disk DB.
        let db = DiskDb::<SimpleCommitment>::open_with_priority(
            app_dir.data_dir(),
            cfg.grug.priority_range.clone(),
        )?;

        // We need to call `RustVm::genesis_codes()` to properly build the contract wrappers.
        let _codes = RustVm::genesis_codes();

        // Create Rust VM.
        let vm = RustVm::new(
            // Below are parameters if we want to switch to `HybridVm`:
            // cfg.grug.wasm_cache_capacity,
            // [
            //     codes.account_factory.to_bytes().hash256(),
            //     codes.account_multi.to_bytes().hash256(),
            //     codes.account_single.to_bytes().hash256(),
            //     codes.bank.to_bytes().hash256(),
            //     codes.dex.to_bytes().hash256(),
            //     codes.gateway.to_bytes().hash256(),
            //     codes.hyperlane.ism.to_bytes().hash256(),
            //     codes.hyperlane.mailbox.to_bytes().hash256(),
            //     codes.hyperlane.va.to_bytes().hash256(),
            //     codes.oracle.to_bytes().hash256(),
            //     codes.taxman.to_bytes().hash256(),
            //     codes.vesting.to_bytes().hash256(),
            //     codes.warp.to_bytes().hash256(),
            // ]
        );

        // Create the base app instance for HTTP server
        let app = App::new(
            db.clone(),
            vm.clone(),
            NaiveProposalPreparer,
            NullIndexer,
            cfg.grug.query_gas_limit,
            None, // the `App` instance for use in httpd doesn't need the upgrade handler
            env!("CARGO_PKG_VERSION"),
        );

        let app = Arc::new(app);

        let (hooked_indexer, _, dango_httpd_context) = self
            .setup_indexer_stack(app_dir, &cfg, app.clone(), &cfg.tendermint.rpc_addr)
            .await?;

        let indexer_clone = hooked_indexer.clone();

        // Create shutdown flags for HTTP servers (to return 503 during shutdown)
        let httpd_shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let httpd_shutdown_flags = vec![httpd_shutdown_flag.clone()];

        // Run ABCI server, optionally with indexer and httpd server.
        match (
            cfg.indexer.enabled,
            cfg.httpd.enabled,
            cfg.metrics_httpd.enabled,
        ) {
            (true, true, true) => {
                // Indexer, HTTP server, and metrics server all enabled
                tokio::try_join!(
                    Self::run_dango_httpd_server(
                        &cfg.httpd,
                        dango_httpd_context,
                        httpd_shutdown_flag.clone()
                    ),
                    Self::run_metrics_httpd_server(&cfg.metrics_httpd, metrics_handler),
                    self.run_with_indexer(
                        cfg.grug,
                        cfg.tendermint,
                        cfg.pyth,
                        db,
                        vm,
                        hooked_indexer.clone(),
                        hooked_indexer,
                        httpd_shutdown_flags
                    )
                )?;
            },
            (true, true, false) => {
                // Indexer and HTTP server enabled, metrics disabled
                tokio::try_join!(
                    Self::run_dango_httpd_server(
                        &cfg.httpd,
                        dango_httpd_context,
                        httpd_shutdown_flag.clone()
                    ),
                    self.run_with_indexer(
                        cfg.grug,
                        cfg.tendermint,
                        cfg.pyth,
                        db,
                        vm,
                        hooked_indexer.clone(),
                        hooked_indexer,
                        httpd_shutdown_flags
                    )
                )?;
            },
            (true, false, true) => {
                // Indexer and metrics enabled, HTTP server disabled
                tokio::try_join!(
                    Self::run_metrics_httpd_server(&cfg.metrics_httpd, metrics_handler),
                    self.run_with_indexer(
                        cfg.grug,
                        cfg.tendermint,
                        cfg.pyth,
                        db,
                        vm,
                        hooked_indexer.clone(),
                        hooked_indexer,
                        vec![] // No HTTP server shutdown flags
                    )
                )?;
            },
            (true, false, false) => {
                // Only indexer enabled
                self.run_with_indexer(
                    cfg.grug,
                    cfg.tendermint,
                    cfg.pyth,
                    db,
                    vm,
                    hooked_indexer.clone(),
                    hooked_indexer,
                    vec![], // No HTTP server shutdown flags
                )
                .await?;
            },
            (false, true, false) => {
                // No indexer, but HTTP server enabled (minimal mode), metrics disabled
                let httpd_context = HttpdContext::new(app);

                tokio::try_join!(
                    Self::run_minimal_httpd_server(
                        &cfg.httpd,
                        httpd_context,
                        httpd_shutdown_flag.clone()
                    ),
                    self.run_with_indexer(
                        cfg.grug,
                        cfg.tendermint,
                        cfg.pyth,
                        db,
                        vm,
                        NullIndexer,
                        NullIndexer,
                        httpd_shutdown_flags
                    )
                )?;
            },
            (false, true, true) => {
                // No indexer, but HTTP server enabled (minimal mode), metrics enabled
                let httpd_context = HttpdContext::new(app);

                tokio::try_join!(
                    Self::run_minimal_httpd_server(
                        &cfg.httpd,
                        httpd_context,
                        httpd_shutdown_flag.clone()
                    ),
                    self.run_with_indexer(
                        cfg.grug,
                        cfg.tendermint,
                        cfg.pyth,
                        db,
                        vm,
                        NullIndexer,
                        NullIndexer,
                        httpd_shutdown_flags
                    ),
                    Self::run_metrics_httpd_server(&cfg.metrics_httpd, metrics_handler)
                )?;
            },
            (false, false, _) => {
                // No indexer, no HTTP server
                self.run_with_indexer(
                    cfg.grug,
                    cfg.tendermint,
                    cfg.pyth,
                    db,
                    vm,
                    NullIndexer,
                    NullIndexer,
                    vec![], // No HTTP server shutdown flags
                )
                .await?;
            },
        }

        indexer_clone.wait_for_finish().await?;

        Ok(())
    }

    /// Setup the hooked indexer with both SQL and Dango indexers, and prepare contexts for HTTP servers
    async fn setup_indexer_stack(
        &self,
        app_dir: HomeDirectory,
        cfg: &Config,
        app: Arc<App<DiskDb<SimpleCommitment>, RustVm, NaiveProposalPreparer, NullIndexer>>,
        tendermint_rpc_addr: &str,
    ) -> anyhow::Result<(
        HookedIndexer,
        indexer_httpd::context::Context,
        dango_httpd::context::Context,
    )> {
        let mut hooked_indexer = HookedIndexer::new();

        let sql_indexer = indexer_sql::IndexerBuilder::default()
            .with_database_url(&cfg.indexer.database.url)
            .with_database_max_connections(cfg.indexer.database.max_connections)
            .with_sqlx_pubsub()
            .build()
            .await
            .map_err(|err| anyhow!("failed to build indexer: {err:?}"))?;
        let indexer_context = sql_indexer.context.clone();

        // Create a separate context for dango indexer (shares DB but has independent pubsub)
        let dango_context: dango_indexer_sql::context::Context = sql_indexer
            .context
            .with_separate_pubsub()
            .await
            .map_err(|e| anyhow!("Failed to create separate context for dango indexer: {e}"))?
            .into();

        let dango_indexer = dango_indexer_sql::indexer::Indexer::new(dango_context.clone());

        let clickhouse_context = dango_indexer_clickhouse::context::Context::new(
            cfg.indexer.clickhouse.url.clone(),
            cfg.indexer.clickhouse.database.clone(),
            cfg.indexer.clickhouse.user.clone(),
            cfg.indexer.clickhouse.password.clone(),
        );

        let clickhouse_indexer = dango_indexer_clickhouse::Indexer::new(clickhouse_context.clone());

        // Create cache indexer (RuntimeHandler no longer needed)
        let mut indexer_cache = indexer_cache::Cache::new_with_dir(app_dir.indexer_dir());
        // Pass S3 config to the cache indexer context
        indexer_cache.context.s3 = cfg.indexer.s3.clone();
        let indexer_cache_context = indexer_cache.context.clone();

        hooked_indexer.add_indexer(indexer_cache).await?;
        hooked_indexer.add_indexer(sql_indexer).await?;
        hooked_indexer.add_indexer(dango_indexer).await?;
        hooked_indexer.add_indexer(clickhouse_indexer).await?;

        let indexer_httpd_context = indexer_httpd::context::Context::new(
            indexer_cache_context,
            indexer_context,
            app.clone(),
            Arc::new(TendermintRpcClient::new(tendermint_rpc_addr)?),
        );

        let dango_httpd_context = dango_httpd::context::Context::new(
            indexer_httpd_context.clone(),
            clickhouse_context.clone(),
            dango_context,
            cfg.httpd.static_files_path.clone(),
        );

        let storage = app
            .db
            .state_storage_with_comment(None, "hooked_indexer")
            .map_err(|e| anyhow!("Failed to get state storage: {e}"))?;
        hooked_indexer
            .start(&storage)
            .await
            .map_err(|e| anyhow!("Failed to start indexer: {e}"))?;

        Ok((hooked_indexer, indexer_httpd_context, dango_httpd_context))
    }

    /// Run the minimal HTTP server (without indexer features)
    /// The shutdown flag should be set when signals are received to return 503 for new requests.
    async fn run_minimal_httpd_server(
        cfg: &HttpdConfig,
        context: HttpdContext,
        shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<()> {
        tracing::info!(cfg.ip, cfg.port, "Starting minimal HTTP server");

        grug_httpd::server::run_server(
            &cfg.ip,
            cfg.port,
            cfg.cors_allowed_origin.clone(),
            context,
            grug_httpd::server::config_app,
            grug_httpd::graphql::build_schema,
            shutdown_flag,
        )
        .await
        .map_err(|err| {
            tracing::error!("Failed to run minimal HTTP server: {err:?}");
            anyhow::anyhow!("Failed to run minimal HTTP server: {err:?}")
        })?;
        Ok(())
    }

    /// Run the full-featured HTTP server (with indexer features)
    /// The shutdown flag should be set when signals are received to return 503 for new requests.
    async fn run_dango_httpd_server(
        cfg: &HttpdConfig,
        dango_httpd_context: dango_httpd::context::Context,
        shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<()> {
        tracing::info!(
            "Starting full-featured HTTP server at {}:{}",
            &cfg.ip,
            cfg.port
        );

        dango_httpd_context
            .indexer_clickhouse_context
            .start_cache()
            .await?;

        dango_httpd::server::run_server(
            &cfg.ip,
            cfg.port,
            cfg.cors_allowed_origin.clone(),
            dango_httpd_context,
            shutdown_flag,
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

    /// Reference:
    /// - Namada:
    ///   https://github.com/namada-net/namada/blob/v101.1.4/crates/node/src/lib.rs#L737-L774
    /// - Penumbra:
    ///   https://github.com/penumbra-zone/penumbra/blob/dafaa19109fd06b67cb294a097bad803ade4ac7c/crates/core/app/src/server.rs#L47-L73
    async fn run_with_indexer<ID>(
        self,
        grug_cfg: GrugConfig,
        tendermint_cfg: TendermintConfig,
        pyth_lazer_cfg: PythLazerConfig,
        db: DiskDb<SimpleCommitment>,
        vm: RustVm,
        indexer: ID,
        mut indexer_for_shutdown: ID,
        httpd_shutdown_flags: Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> anyhow::Result<()>
    where
        ID: Indexer + Send + 'static,
    {
        let app = App::new(
            db,
            vm,
            ProposalPreparer::new(pyth_lazer_cfg.endpoints, pyth_lazer_cfg.access_token),
            indexer,
            grug_cfg.query_gas_limit,
            Some(dango_upgrade::do_upgrade), // Important: set the upgrade handler.
            env!("CARGO_PKG_VERSION"),
        );

        let (consensus, mempool, snapshot, info) = split::service(app, 1);

        let abci_server = Server::builder()
            .consensus(consensus)
            .snapshot(snapshot)
            .mempool(mempool)
            .info(info)
            .finish()
            // Safety: the consensus, snapshot, mempool, and info services have all been provided
            // to the builder above.
            .expect("all components of abci have been provided");

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
                // Set shutdown flags to return 503 for new HTTP requests
                for flag in &httpd_shutdown_flags {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                // Give a brief moment for the flags to propagate
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Err(err) = indexer_for_shutdown.shutdown().await {
                    tracing::error!(err = %err, "Error shutting down indexer");
                }
                telemetry::shutdown();
                telemetry::shutdown_sentry();
                Ok(())
            },
            _ = sigterm.recv() => {
                tracing::info!("Received SIGTERM, shutting down");
                // Set shutdown flags to return 503 for new HTTP requests
                for flag in &httpd_shutdown_flags {
                    flag.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                // Give a brief moment for the flags to propagate
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Err(err) = indexer_for_shutdown.shutdown().await {
                    tracing::error!(err = %err, "Error shutting down indexer");
                }
                telemetry::shutdown();
                telemetry::shutdown_sentry();
                Ok(())
            },
        }
    }
}
