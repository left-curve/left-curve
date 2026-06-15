use {
    crate::projection_loop,
    anyhow::Context,
    dango_indexer_historical_block_source::BlockSource,
    dango_indexer_historical_projection::{Committer, Projection},
    dango_indexer_historical_types::AnyResult,
    futures::future::try_join_all,
    std::sync::Arc,
};

/// Top-level orchestrator: owns a single [`BlockSource`], the [`Committer`]
/// shared by all projections, and a fixed set of [`Projection`]s. Spawns
/// the source's `run()` task plus one [`projection_loop`] per projection,
/// and waits for all of them.
///
/// Construct with [`App::new`] and drive with [`App::run`].
pub struct App {
    source: Arc<dyn BlockSource>,
    committer: Arc<dyn Committer>,
    projections: Vec<Arc<dyn Projection>>,
}

impl App {
    pub fn new(
        source: Arc<dyn BlockSource>,
        committer: Arc<dyn Committer>,
        projections: Vec<Arc<dyn Projection>>,
    ) -> Self {
        Self {
            source,
            committer,
            projections,
        }
    }

    /// Migrate storage, then spawn the source + all projection loops and
    /// wait for them.
    ///
    /// Returns when any task finishes (cleanly or with error). The current
    /// `LocalBlockSource::run` loops forever, so in practice this only
    /// returns on a panic or an unrecoverable error from one of the tasks.
    pub async fn run(&self) -> AnyResult<()> {
        // Boot: the committer derives every owner's migrations from the
        // registered projections and applies them before any task starts.
        self.committer.migrate(&self.projections).await?;

        let mut handles = Vec::with_capacity(self.projections.len() + 1);

        handles.push(tokio::spawn(self.source.clone().run()));

        for p in &self.projections {
            handles.push(tokio::spawn(projection_loop(
                p.clone(),
                self.source.clone(),
                self.committer.clone(),
            )));
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
