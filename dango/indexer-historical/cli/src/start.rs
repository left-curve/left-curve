use {
    crate::{config::Config, home_directory::HomeDirectory, source},
    clap::Parser,
    dango_config_parser::parse_config,
};

/// `start` — boot the historical indexer.
///
/// Assembles the composition root and supervises it until a shutdown signal:
///
/// 1. parse the config from `home.config_file()`;
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
    pub async fn run(self, home: HomeDirectory) -> anyhow::Result<()> {
        tracing::info!(
            home = %home.display(),
            commit = dango_primitives::GIT_COMMIT,
            "starting the historical indexer",
        );

        // Step 1: parse the config (TOML + `SECTION__FIELD` env overrides).
        let cfg: Config = parse_config(home.config_file())?;
        // A secrets-safe summary — never the Postgres URL or the source URLs.
        tracing::info!(
            block_source = cfg.block_source.kind(),
            fetcher = cfg.block_source.fetcher_kind().unwrap_or("none"),
            httpd_enabled = cfg.httpd.enabled,
            httpd_bind = %cfg.httpd.bind,
            pg_max_connections = cfg.postgres.max_connections,
            "configuration loaded",
        );

        // Step 2: build the configured block source. The app, committer, and
        // read schema all see it only as `Arc<dyn BlockSource>`.
        let block_source = source::build(&cfg.block_source, &home)?;
        let frontier = block_source.contiguous_frontier().await?;
        tracing::info!(?frontier, "block source built");

        // TODO: wire the composition root (steps 3–6: committer, projections,
        // schema, app) and replace this with `app.run().await`.
        anyhow::bail!("`start` is not wired beyond the block source yet");
    }
}
