use {
    crate::{
        block_and_outcome::BlockAndOutcome,
        context::Context,
        perps_events::{PerpsEventBlock, extract_perps_event_block},
        recent_stream::RecentStream,
    },
    async_trait::async_trait,
    dango_app::IndexerResult,
    dango_primitives::{Block, BlockOutcome, Config, Json, JsonDeExt},
    dango_types::config::AppConfig,
    std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    },
};

/// Number of recent blocks the realtime stream retains in memory — both the
/// reconnect/recovery window for `perps_events2` and the live broadcast buffer.
///
/// Bounds memory (one `PerpsEventBlock` per height, including empty ones) and
/// the maximum reconnect depth. Deeper history is the indexer node's job (the
/// durable `perps_events` table / `perpsEvents` query), not this ephemeral,
/// validator-side feed. Promote to a config field if ops needs to tune it.
pub const DEFAULT_RING_CAPACITY: usize = 1000;

/// Number of recent full blocks the `full_block` stream retains in memory — both
/// the reconnect/recovery window and the live broadcast buffer.
///
/// Smaller than [`DEFAULT_RING_CAPACITY`] on purpose: a `BlockAndOutcome` (every
/// transaction + every event of a block) is far larger than a `PerpsEventBlock`,
/// and one is retained per height whether or not anyone is subscribed. Deeper
/// history is the indexer node's job (the on-disk block files and the REST
/// `/block/*` routes), not this ephemeral validator-side window. Promote to a
/// config field if ops needs to tune it.
pub const DEFAULT_BLOCK_RING_CAPACITY: usize = 100;

/// The validator-side realtime indexer. Despite implementing
/// [`dango_app::Indexer`], it does no durable indexing: it maintains an
/// in-memory ring of recent perps-contract events and broadcasts them to
/// `perps_events2` subscribers, entirely in-process (lowest latency, no
/// validator -> indexer-node hop).
///
/// FUTURE: this crate will gain a second [`RecentStream`] — `RecentStream<..>`
/// over full blocks — to serve a "new blocks" subscription consumed by the
/// indexer node. The [`RecentStream`] primitive is generic precisely so that
/// drops onto the same machinery.
///
/// FUTURE: once block files / Postgres / ClickHouse move to a dedicated indexer
/// node, this indexer will REPLACE `HookedIndexer` as the validator's sole,
/// thin indexer. For now it rides alongside the others as a `HookedIndexer`
/// field.
#[derive(Clone)]
pub struct Indexer {
    inner: Arc<Inner>,
}

struct Inner {
    perps: RecentStream<PerpsEventBlock>,

    /// In-memory ring + live broadcast of full blocks (`Block` + `BlockOutcome`)
    /// backing the `full_block` subscription. Fed in `index_block`, in strict
    /// height order; see the crate docs.
    blocks: RecentStream<BlockAndOutcome>,

    /// `index_block` -> `post_indexing` hand-off. The `Indexer` trait's
    /// `post_indexing` does not receive the `BlockOutcome` (only `app_cfg`,
    /// which carries the perps address), so we stash the outcome + block
    /// timestamp at `index_block` and drain it at `post_indexing`. At most one
    /// entry is in flight: CometBFT runs FinalizeBlock(N) then Commit(N) before
    /// FinalizeBlock(N+1). The lock is held only for the map op, never across
    /// an `.await`.
    pending: Mutex<HashMap<u64, PendingBlock>>,
}

struct PendingBlock {
    created_at: String,
    outcome: BlockOutcome,
}

impl Indexer {
    /// `perps_ring_capacity` and `block_ring_capacity` size the two in-memory
    /// rings (reconnect window + broadcast buffer) for the `perps_events2` and
    /// `full_block` subscriptions respectively. See [`DEFAULT_RING_CAPACITY`]
    /// and [`DEFAULT_BLOCK_RING_CAPACITY`].
    pub fn new(perps_ring_capacity: usize, block_ring_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                perps: RecentStream::new(perps_ring_capacity),
                blocks: RecentStream::new(block_ring_capacity),
                pending: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// A cheap-to-clone reader handle for the httpd server.
    pub fn context(&self) -> Context {
        Context::new(self.inner.perps.clone(), self.inner.blocks.clone())
    }
}

#[async_trait]
impl dango_app::Indexer for Indexer {
    fn name(&self) -> &'static str {
        "dango-indexer-stream"
    }

    // `start`, `shutdown`, `pre_indexing`, `wait_for_finish` use the trait's
    // default no-op impls: there is no durable store to migrate or drain.

    async fn index_block(&self, block: &Block, block_outcome: &BlockOutcome) -> IndexerResult<()> {
        // Full-block feed: broadcast the finalized block + outcome to
        // `full_block` subscribers. `index_block` is awaited by the app once per
        // height in strict order (CometBFT runs FinalizeBlock(N) before
        // FinalizeBlock(N+1)), so appending here keeps the ring monotonic and
        // gap-free — and it is the earliest point at which the full block is in
        // hand. The cold `reindex` path skips `index_block`, so the ring stays
        // live-only, exactly like the perps feed below.
        self.inner.blocks.append(Arc::new(BlockAndOutcome {
            block: block.clone(),
            block_outcome: block_outcome.clone(),
        }));

        #[cfg(feature = "metrics")]
        metrics::counter!("indexer_stream.full_blocks.published.total").increment(1);

        // Perps feed: stash only. The perps address lives in `app_cfg`, which
        // `index_block` does not receive, so the actual extraction + publish
        // happens INLINE and in height order in `post_indexing` (see
        // HookedIndexer).
        let pending = PendingBlock {
            created_at: block.info.timestamp.to_rfc3339_string(),
            outcome: block_outcome.clone(),
        };

        self.inner
            .pending
            .lock()
            .unwrap()
            .insert(block.info.height, pending);

        Ok(())
    }

    async fn post_indexing(
        &self,
        block_height: u64,
        _cfg: Config,
        app_cfg: Json,
    ) -> IndexerResult<()> {
        let Some(PendingBlock {
            created_at,
            outcome,
        }) = self.inner.pending.lock().unwrap().remove(&block_height)
        else {
            // No stash — e.g. the `reindex` cold-catch-up path skips
            // `index_block`. The realtime feed is live-only + in-memory; there
            // is nothing to backfill here, and no live subscribers during a
            // cold catch-up. Intentionally a no-op.
            return Ok(());
        };

        // Best-effort: a malformed/absent `app_cfg` (e.g. grug-only test
        // harnesses that don't write APP_CONFIG) must never abort indexing or
        // panic consensus. Log and skip this block's events.
        let app_cfg: AppConfig = match app_cfg.deserialize_json() {
            Ok(cfg) => cfg,
            Err(_err) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    block_height,
                    err = %_err,
                    "skipping perps_events2 publish: app_cfg deserialize failed"
                );

                return Ok(());
            },
        };

        let batch =
            extract_perps_event_block(block_height, created_at, outcome, app_cfg.addresses.perps);

        #[cfg(feature = "metrics")]
        {
            metrics::counter!("indexer_stream.blocks.published.total").increment(1);

            metrics::counter!("indexer_stream.events.published.total")
                .increment(batch.events.len() as u64);
        }

        // Append every block (including empty ones) so subscriber heights stay
        // contiguous and gap detection stays exact.
        self.inner.perps.append(Arc::new(batch));

        Ok(())
    }

    async fn last_indexed_block_height(&self) -> IndexerResult<Option<u64>> {
        Ok(self.inner.perps.tip())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_app::Indexer as _,
        dango_primitives::{BlockInfo, Hash256, Timestamp},
        futures_util::stream::StreamExt,
    };

    fn block_and_outcome(height: u64) -> (Block, BlockOutcome) {
        let block = Block {
            info: BlockInfo {
                height,
                timestamp: Timestamp::from_seconds(100),
                hash: Hash256::ZERO,
            },
            txs: vec![],
        };

        let block_outcome = BlockOutcome {
            height,
            app_hash: Hash256::ZERO,
            cron_outcomes: vec![],
            tx_outcomes: vec![],
        };

        (block, block_outcome)
    }

    /// `index_block` feeds the full-block ring in strict height order, so a
    /// `full_block` subscriber (reading through `context().blocks()`) receives
    /// every block exactly once, ascending — no gaps, no duplicates.
    #[tokio::test]
    async fn index_block_feeds_full_block_ring_in_order() {
        let indexer = Indexer::new(DEFAULT_RING_CAPACITY, DEFAULT_BLOCK_RING_CAPACITY);
        let ctx = indexer.context();

        // Subscribe before any block is appended (empty ring): a live feed from
        // height 1 onward.
        let stream = ctx
            .blocks()
            .subscribe(Some(1), |b: &BlockAndOutcome| Some(b.block.info.height))
            .unwrap();

        for height in 1..=3 {
            let (block, block_outcome) = block_and_outcome(height);
            indexer.index_block(&block, &block_outcome).await.unwrap();
        }

        let got: Vec<u64> = stream.take(3).collect().await;
        assert_eq!(got, vec![1, 2, 3]);
    }
}
