use {
    crate::projection_loop,
    anyhow::Context,
    dango_indexer_historical_block_source::BlockSource,
    dango_indexer_historical_projection::{Committer, Projection},
    dango_indexer_historical_types::AnyResult,
    futures::future::{BoxFuture, try_join_all},
    std::sync::Arc,
};

/// Top-level orchestrator: owns a single [`BlockSource`], the [`Committer`]
/// shared by all projections, and a fixed set of [`Projection`]s. Spawns the
/// source's `run()` task, one [`projection_loop`] per projection, and — when
/// one is given — the read-API server task, then waits for all of them.
///
/// The read API is passed in **already built**: the schema, assembled from the
/// projections' query / subscription objects, is baked into the task by the
/// composition root. The app stays agnostic — it neither defines the schema nor
/// knows its concrete type; it just supervises one more future.
///
/// Construct with [`App::new`] and drive with [`App::run`].
pub struct App {
    source: Arc<dyn BlockSource>,
    committer: Arc<dyn Committer>,
    projections: Vec<Arc<dyn Projection>>,
    /// Pre-built read-API server task (`None` = ingest-only), supervised
    /// alongside the ingest tasks and consumed on `run`.
    httpd: Option<BoxFuture<'static, AnyResult<()>>>,
}

impl App {
    pub fn new(
        source: Arc<dyn BlockSource>,
        committer: Arc<dyn Committer>,
        projections: Vec<Arc<dyn Projection>>,
        httpd: Option<BoxFuture<'static, AnyResult<()>>>,
    ) -> Self {
        Self {
            source,
            committer,
            projections,
            httpd,
        }
    }

    /// Migrate storage, then spawn the source, all projection loops, and the
    /// read-API server (if any), and wait for them.
    ///
    /// Returns when any task finishes (cleanly or with error). The ingest tasks
    /// and the server loop forever, so in practice this only returns on a panic
    /// or an unrecoverable error from one of the tasks.
    pub async fn run(self) -> AnyResult<()> {
        // Boot: the committer derives every owner's migrations from the
        // registered projections and applies them before any task starts.
        self.committer.migrate(&self.projections).await?;

        let mut handles = Vec::with_capacity(self.projections.len() + 2);

        handles.push(tokio::spawn(self.source.clone().run()));

        for p in &self.projections {
            handles.push(tokio::spawn(projection_loop(
                p.clone(),
                self.source.clone(),
                self.committer.clone(),
            )));
        }

        // Serve the read API from the same process that ingests: one more
        // supervised task, so a server error stops the app too.
        if let Some(httpd) = self.httpd {
            handles.push(tokio::spawn(httpd));
        }

        // try_join_all surfaces the first join error (panic / cancellation);
        // we then bubble up the first inner task error if any.
        let results = try_join_all(handles).await.context("task join")?;
        for r in results {
            r?;
        }
        Ok(())
    }
}
