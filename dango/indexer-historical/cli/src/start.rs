use {
    crate::{activity, config::Config, db, home_directory::HomeDirectory, read_api, source},
    clap::Parser,
    dango_indexer_historical_app::{App, PgChCommitter},
    dango_indexer_historical_projection::{ActivityProjection, Committer, Projection},
    std::sync::Arc,
};

/// `start` — boot the historical indexer.
///
/// Assembles the composition root and supervises it until a shutdown signal:
///
/// 1. reuse the `cfg` parsed in `main` (used there to set up tracing);
/// 2. build the configured `BlockSource` (local / remote) as an
///    `Arc<dyn BlockSource>` — the rest of the app is agnostic to which;
/// 3. open the Postgres pool and build the shared `Committer`;
/// 4. assemble the registered projections (`ActivityProjection`, …);
/// 5. build the read schema — `Query(BlockQuery, ActivityQuery)` via the
///    httpd's `build_schema` — and wrap it in the `serve(…)` task;
/// 6. hand them to `App::new` and `run()` the supervisor.
#[derive(Parser)]
pub struct StartCmd;

impl StartCmd {
    pub async fn run(self, home: HomeDirectory, cfg: Config) -> anyhow::Result<()> {
        // Install the global Prometheus recorder first, so metrics emitted during
        // startup are captured. Recording is always on; serving is gated below.
        let metrics_handle = crate::metrics::install()?;

        tracing::info!(
            home = %home.display(),
            commit = dango_primitives::GIT_COMMIT,
            "starting the historical indexer",
        );

        // The config was parsed in `main` (to set up tracing); reuse it here.
        // A secrets-safe summary — never the Postgres URL or the source URLs.
        tracing::info!(
            block_source = cfg.block_source.kind(),
            fetcher = cfg.block_source.fetcher_kind().unwrap_or("none"),
            httpd_enabled = cfg.httpd.enabled,
            httpd_bind = %cfg.httpd.bind,
            metrics_enabled = cfg.metrics.enabled,
            metrics_bind = %format!("{}:{}", cfg.metrics.ip, cfg.metrics.port),
            pg_max_connections = cfg.postgres.max_connections,
            "configuration loaded",
        );

        // Step 2: build the configured block source. The app, committer, and
        // read schema all see it only as `Arc<dyn BlockSource>`.
        let block_source = source::build(&cfg.block_source, &home)?;
        let frontier = block_source.contiguous_frontier().await?;
        tracing::info!(?frontier, "block source built");

        // Step 3: open Postgres and assemble the shared committer + the
        // compiled-in set of projections (ClickHouse deferred → `None`).
        let db = db::connect(&cfg.postgres).await?;
        let committer: Arc<dyn Committer> = Arc::new(PgChCommitter::new(db.clone(), None));

        // The activity config harvests the deployment's system contracts from
        // the node's `app_config` (queried with retry) to seed the participation
        // blacklist, merged with any config addresses.
        let activity_cfg = activity::config(&cfg.activity, cfg.block_source.node_url()).await?;
        let projections: Vec<Arc<dyn Projection>> =
            vec![Arc::new(ActivityProjection::new(activity_cfg))];
        tracing::info!(
            projections = projections.len(),
            "committer and projections ready"
        );

        // Step 4-5: the GraphQL read API over the shared pool + block source —
        // `None` when disabled, which runs the indexer ingest-only.
        let httpd = read_api::task(&cfg.httpd, db, block_source.clone());
        tracing::info!(read_api = httpd.is_some(), "read API task assembled");

        // Step 6: supervise the source, one loop per projection, and (when
        // enabled) the read API, until a task ends. `App::run` migrates first.
        let app = App::new(block_source, committer, projections, httpd);

        // Supervise the ingest+read-API app alongside the Prometheus endpoint.
        // `try_join!` polls both on this task (no `Send` wrapper needed for the
        // actix metrics server), and the first to finish/err tears the other
        // down. With metrics disabled it is just `App::run`.
        if cfg.metrics.enabled {
            tracing::info!(
                metrics_bind = %format!("{}:{}", cfg.metrics.ip, cfg.metrics.port),
                "serving the /metrics endpoint"
            );
            tokio::try_join!(
                app.run(),
                crate::metrics::serve(&cfg.metrics, metrics_handle)
            )?;
            Ok(())
        } else {
            app.run().await
        }
    }
}
