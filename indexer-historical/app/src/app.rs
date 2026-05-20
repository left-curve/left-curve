use {
    crate::projection_loop, anyhow::Context, futures::future::try_join_all,
    indexer_historical_block_source::BlockSource, indexer_historical_projection::Projection,
    indexer_historical_types::AnyResult, std::sync::Arc,
};

/// Top-level orchestrator: owns a single [`BlockSource`] and a fixed set of
/// [`Projection`]s, spawns the source's `run()` task plus one
/// [`projection_loop`] per projection, and waits for all of them.
///
/// Construct with [`App::new`] and drive with [`App::run`].
pub struct App {
    source: Arc<dyn BlockSource>,
    projections: Vec<Arc<dyn Projection>>,
}

impl App {
    pub fn new(source: Arc<dyn BlockSource>, projections: Vec<Arc<dyn Projection>>) -> Self {
        Self {
            source,
            projections,
        }
    }

    /// Spawn the source + all projection loops and wait for them.
    ///
    /// Returns when any task finishes (cleanly or with error). The current
    /// `LocalBlockSource::run` loops forever, so in practice this only
    /// returns on a panic or an unrecoverable error from one of the tasks.
    pub async fn run(&self) -> AnyResult<()> {
        let mut handles = Vec::with_capacity(self.projections.len() + 1);

        handles.push(tokio::spawn(self.source.clone().run()));

        for p in &self.projections {
            handles.push(tokio::spawn(projection_loop(
                p.clone(),
                self.source.clone(),
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
