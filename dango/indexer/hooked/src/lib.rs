use {
    async_trait::async_trait,
    dango_app::{APP_CONFIG, CONFIG, Indexer, IndexerResult, LAST_FINALIZED_BLOCK},
    dango_primitives::{Block, BlockOutcome, Config, Json, Storage},
    std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    },
    tokio::{sync::Mutex, time::sleep},
};

// Re-export for convenience
pub use dango_app::IndexerError;

/// Composite indexer that owns the three production indexer components and
/// orchestrates their per-block work as a single `dango_app::Indexer`.
///
/// The "Hooked" name is historical — earlier revisions of this crate held a
/// dynamic `Arc<RwLock<Vec<Box<dyn Indexer>>>>` and broadcasted each trait
/// method to its entries, plumbing data between them through an opaque
/// `http::Extensions`-based `IndexerContext`. The current shape is three
/// concrete fields with the data flow expressed by typed method arguments,
/// but the crate and struct name are kept so callers, deploy scripts, and
/// imports do not need to churn.
///
/// `post_indexing` is still spawned as a separate tokio task per block so
/// SQL and Clickhouse writes do not block consensus; `wait_for_finish` drains
/// the per-block task map before letting the binary exit.
#[derive(Clone)]
pub struct HookedIndexer {
    pub file: dango_indexer_cache::Cache,
    pub sql: dango_indexer_sql::Indexer,
    pub clickhouse: dango_indexer_clickhouse::Indexer,
    /// Set to true between `start` and `shutdown`.
    is_running: Arc<AtomicBool>,
    /// One join handle per in-flight `post_indexing` block. Inserted when the
    /// task is spawned; removed by the task itself when it completes.
    post_indexing_tasks: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
}

impl HookedIndexer {
    pub fn new(
        file: dango_indexer_cache::Cache,
        sql: dango_indexer_sql::Indexer,
        clickhouse: dango_indexer_clickhouse::Indexer,
    ) -> Self {
        Self {
            file,
            sql,
            clickhouse,
            is_running: Arc::new(AtomicBool::new(false)),
            post_indexing_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if the indexer is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Replay any blocks that the SQL or Clickhouse stores missed since their
    /// last indexed height, up to `LAST_FINALIZED_BLOCK`. `index_block` is
    /// intentionally skipped — the cached on-disk file is already there, so
    /// `pre_indexing` reloads the payload and `post_indexing` flushes it to
    /// the SQL and Clickhouse stores.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn reindex(&self, storage: &dyn Storage) -> IndexerResult<()> {
        let block = match LAST_FINALIZED_BLOCK.load(storage) {
            Err(_err) => {
                // This happens when the chain starts at genesis
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_err, "No `LAST_FINALIZED_BLOCK` found");
                return Ok(());
            },
            Ok(block) => block,
        };

        #[cfg(feature = "tracing")]
        tracing::warn!(
            block_height = block.height,
            "Start called, found LAST_FINALIZED_BLOCK"
        );

        let cfg = CONFIG
            .load(storage)
            .map_err(|e| IndexerError::storage(format!("Failed to load CONFIG: {e}")))?;

        let app_cfg = APP_CONFIG
            .load(storage)
            .map_err(|e| IndexerError::storage(format!("Failed to load APP_CONFIG: {e}")))?;

        // Lowest last-indexed height across components decides where we resume.
        // Cache has no DB-backed counter; only SQL/Clickhouse contribute. (See
        // the inherent `last_indexed_block_height` impls.)
        let sql_last = self.sql.last_indexed_block_height().await.ok().flatten();
        let clickhouse_last = self
            .clickhouse
            .last_indexed_block_height()
            .await
            .ok()
            .flatten();
        let min_height = [sql_last, clickhouse_last]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or_default();

        if min_height >= block.height {
            #[cfg(feature = "tracing")]
            tracing::info!(
                block_height = block.height,
                "No reindexing needed, all indexers are up to date",
            );
            return Ok(());
        }

        for block_height in (min_height + 1)..=block.height {
            // Cache loads the cached file into its in-memory map.
            self.file.pre_indexing(block_height).await?;

            // Skip `index_block`: we don't have the live block data here and
            // it's only used to *write* the disk cache, which already exists.

            // Cache drains its in-memory map for this block and hands the
            // payload to the SQL + Clickhouse consumers.
            let payload = self.file.post_indexing(block_height).await?;
            self.sql
                .post_indexing(block_height, cfg.clone(), app_cfg.clone(), &payload)
                .await?;
            self.clickhouse
                .post_indexing(block_height, cfg.clone(), app_cfg.clone(), &payload)
                .await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Indexer for HookedIndexer {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn start(&mut self, storage: &dyn Storage) -> IndexerResult<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(IndexerError::already_running());
        }

        self.file.start(storage).await?;
        self.sql.start(storage).await?;
        self.clickhouse.start(storage).await?;

        self.reindex(storage).await?;

        self.is_running.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// Shut down in reverse-construction order so any tail work (e.g.
    /// Clickhouse candle flushes in `wait_for_finish`) drains before the
    /// upstream stores close.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn shutdown(&mut self) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(()); // Already shut down
        }

        self.wait_for_finish().await?;

        let mut errors = Vec::new();

        if let Err(err) = self.clickhouse.shutdown().await {
            #[cfg(feature = "tracing")]
            tracing::error!(err = %err, indexer = "clickhouse", "Error in shutdown");
            errors.push(err.to_string());
        }
        if let Err(err) = self.sql.shutdown().await {
            #[cfg(feature = "tracing")]
            tracing::error!(err = %err, indexer = "sql", "Error in shutdown");
            errors.push(err.to_string());
        }
        if let Err(err) = self.file.shutdown().await {
            #[cfg(feature = "tracing")]
            tracing::error!(err = %err, indexer = "file", "Error in shutdown");
            errors.push(err.to_string());
        }

        self.is_running.store(false, Ordering::Relaxed);

        if !errors.is_empty() {
            return Err(IndexerError::multiple(errors));
        }

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn pre_indexing(&self, block_height: u64) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(IndexerError::not_running());
        }

        // Only Cache has work here: it reloads the on-disk file (if any) into
        // its in-memory map so `post_indexing` can drain it later. SQL and
        // Clickhouse do nothing in this phase.
        self.file.pre_indexing(block_height).await?;

        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn index_block(&self, block: &Block, block_outcome: &BlockOutcome) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(IndexerError::not_running());
        }

        // Only Cache has work here: it writes the block to disk (or reloads
        // it if it was already there) and stashes the payload in its
        // in-memory map for `post_indexing`.
        self.file.index_block(block, block_outcome).await?;

        Ok(())
    }

    /// Spawn one tokio task per block. The task drains Cache's in-memory map
    /// (producing the typed payload), then hands the payload to SQL and
    /// Clickhouse in sequence. SQL must run before Clickhouse so the single
    /// pubsub publish at the end of `SQL.post_indexing` fires before
    /// Clickhouse's own publish (preserves the post-phase-2 ordering).
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn post_indexing(
        &self,
        block_height: u64,
        cfg: Config,
        app_cfg: Json,
    ) -> IndexerResult<()> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Err(IndexerError::not_running());
        }

        let file = self.file.clone();
        let sql = self.sql.clone();
        let clickhouse = self.clickhouse.clone();
        let post_indexing_tasks = self.post_indexing_tasks.clone();

        let handle = tokio::spawn(async move {
            let payload = match file.post_indexing(block_height).await {
                Ok(payload) => payload,
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        err = %_err,
                        block_height,
                        "Cache post_indexing failed; skipping SQL + Clickhouse writes"
                    );

                    // Cleanup is still required so `wait_for_finish` terminates.
                    post_indexing_tasks.lock().await.remove(&block_height);
                    return;
                },
            };

            if let Err(_err) = sql
                .post_indexing(block_height, cfg.clone(), app_cfg.clone(), &payload)
                .await
            {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    err = %_err,
                    block_height,
                    "SQL post_indexing failed"
                );
            }

            if let Err(_err) = clickhouse
                .post_indexing(block_height, cfg, app_cfg, &payload)
                .await
            {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    err = %_err,
                    block_height,
                    "Clickhouse post_indexing failed"
                );
            }

            // Remove our handle so `wait_for_finish` can terminate.
            post_indexing_tasks.lock().await.remove(&block_height);
        });

        self.post_indexing_tasks
            .lock()
            .await
            .insert(block_height, handle);

        Ok(())
    }

    /// Wait for in-flight `post_indexing` tasks to finish, then forward to the
    /// three components' own `wait_for_finish` hooks. Same shape as the old
    /// dyn-dispatch version: poll the task map every 100ms.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn wait_for_finish(&self) -> IndexerResult<()> {
        #[cfg(feature = "tracing")]
        tracing::debug!("Waiting for indexer to finish");

        // 1. Drain in-flight per-block `post_indexing` tasks.
        loop {
            let tasks_guard = self.post_indexing_tasks.lock().await;

            if tasks_guard.is_empty() {
                break;
            }

            #[cfg(feature = "tracing")]
            let block_heights: Vec<u64> = tasks_guard.keys().copied().collect();
            #[cfg(feature = "tracing")]
            let task_count = tasks_guard.len();
            drop(tasks_guard);

            #[cfg(feature = "tracing")]
            tracing::debug!(
                tasks = task_count,
                blocks = ?block_heights,
                "Waiting for post_indexing tasks to finish"
            );

            sleep(Duration::from_millis(100)).await;

            let mut tasks_guard = self.post_indexing_tasks.lock().await;
            tasks_guard.retain(|&block_height, handle| {
                let finished = handle.is_finished();
                #[cfg(feature = "tracing")]
                if finished {
                    tracing::debug!(block_height, "Post_indexing task completed");
                }
                #[cfg(not(feature = "tracing"))]
                let _ = block_height;
                !finished
            });
        }

        // 2. Drain each component's own background work.
        self.file.wait_for_finish().await?;
        self.sql.wait_for_finish().await?;
        self.clickhouse.wait_for_finish().await?;

        Ok(())
    }

    async fn last_indexed_block_height(&self) -> IndexerResult<Option<u64>> {
        let sql = self.sql.last_indexed_block_height().await?;
        let clickhouse = self.clickhouse.last_indexed_block_height().await?;
        Ok([sql, clickhouse].into_iter().flatten().min())
    }
}

impl Drop for HookedIndexer {
    fn drop(&mut self) {
        // `shutdown` is async; we can't call it from `Drop`. Flip the flag so
        // any concurrent operations bail; the actual cleanup runs through the
        // explicit `shutdown` call in the binary's signal handler.
        self.is_running.store(false, Ordering::Relaxed);
    }
}
