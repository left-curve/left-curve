use {
    crate::{
        context::Context,
        perps_events::{PerpsEventBlock, extract_perps_event_block},
        recent_stream::RecentStream,
    },
    async_trait::async_trait,
    dango_app::IndexerResult,
    dango_primitives::{Addr, Block, BlockOutcome, Config, FullBlock, Json, JsonDeExt},
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
/// Smaller than [`DEFAULT_RING_CAPACITY`] on purpose: a `FullBlock` (every
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
    /// backing the `full_block` subscription. Fed in `post_indexing`
    /// (post-commit), in strict height order; see the crate docs.
    blocks: RecentStream<FullBlock>,

    /// `index_block` -> `post_indexing` hand-off. `index_block` runs at
    /// FinalizeBlock — before the block is committed — and is the only place the
    /// full `Block` + `BlockOutcome` are in hand; `post_indexing` runs after
    /// Commit but receives only `app_cfg` (which carries the perps address). So
    /// we stash the block + outcome here and drain + publish BOTH realtime feeds
    /// at `post_indexing`, once the block is committed — a subscriber never sees
    /// a block the app then fails to commit. At most one entry is in flight:
    /// CometBFT runs FinalizeBlock(N) then Commit(N) before FinalizeBlock(N+1).
    /// The lock is held only for the map op, never across an `.await`.
    pending: Mutex<HashMap<u64, PendingBlock>>,
}

struct PendingBlock {
    block: Block,
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

    /// Drain the `index_block` stash for `block_height` and publish it to the
    /// realtime rings, in strict height order (the app awaits `post_indexing`
    /// per height). Runs after the block is committed. The full-block ring is
    /// always fed; perps events are extracted and published only when
    /// `perps_addr` is known. A no-op if nothing is stashed — the cold `reindex`
    /// path skips `index_block`, so there are no live subscribers to serve.
    fn publish_committed_block(&self, block_height: u64, perps_addr: Option<Addr>) {
        let Some(PendingBlock { block, outcome }) =
            self.inner.pending.lock().unwrap().remove(&block_height)
        else {
            return;
        };

        // Perps feed: extract this block's perps-contract events (only when the
        // perps address is known), before the outcome is moved into the
        // full-block ring below. Append every block — including empty ones — so
        // subscriber heights stay contiguous and gap detection stays exact.
        if let Some(perps_addr) = perps_addr {
            let created_at = block.info.timestamp.to_rfc3339_string();
            let batch =
                extract_perps_event_block(block_height, created_at, outcome.clone(), perps_addr);

            #[cfg(feature = "metrics")]
            {
                metrics::counter!("indexer_stream.blocks.published.total").increment(1);

                metrics::counter!("indexer_stream.events.published.total")
                    .increment(batch.events.len() as u64);
            }

            self.inner.perps.append(Arc::new(batch));
        }

        // Full-block feed: the committed block + outcome, published regardless
        // of `app_cfg`.
        self.inner
            .blocks
            .append(Arc::new(FullBlock { block, outcome }));

        #[cfg(feature = "metrics")]
        metrics::counter!("indexer_stream.full_blocks.published.total").increment(1);
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
        // Stash only. `index_block` runs at FinalizeBlock — the block is
        // finalized but the ABCI app has NOT committed it yet — so we record it
        // here, keyed by height, and publish both realtime feeds from
        // `post_indexing` once the block is committed (see
        // `publish_committed_block`). This keeps a subscriber from ever
        // observing a block the app then fails to commit, and matches how the
        // perps feed has always worked (the perps address it needs only arrives
        // with `app_cfg` at `post_indexing`). The cold `reindex` path skips
        // `index_block`, so the rings stay live-only.
        let pending = PendingBlock {
            block: block.clone(),
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
        // Resolve the perps contract address from `app_cfg`. Best-effort: a
        // malformed/absent `app_cfg` (e.g. grug-only test harnesses that don't
        // write APP_CONFIG) must never abort indexing or panic consensus — we
        // still publish the full block, only the perps extraction is skipped.
        let perps_addr = match app_cfg.deserialize_json::<AppConfig>() {
            Ok(cfg) => Some(cfg.addresses.perps),
            Err(_err) => {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    block_height,
                    err = %_err,
                    "skipping perps_events2 publish: app_cfg deserialize failed"
                );

                None
            },
        };

        self.publish_committed_block(block_height, perps_addr);

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

    /// The full-block feed is published from `post_indexing` (after the block
    /// is committed), NOT from `index_block` (which only stashes the finalized
    /// block). A `full_block` subscriber (reading through `context().blocks()`)
    /// then receives every block exactly once, ascending — no gaps, no
    /// duplicates.
    #[tokio::test]
    async fn full_block_ring_is_published_in_post_indexing_in_order() {
        let indexer = Indexer::new(DEFAULT_RING_CAPACITY, DEFAULT_BLOCK_RING_CAPACITY);
        let ctx = indexer.context();

        // Subscribe before any block is published (empty ring): a live feed from
        // height 1 onward.
        let stream = ctx
            .blocks()
            .subscribe(Some(1), |b: &FullBlock| Some(b.block.info.height))
            .unwrap();

        // `index_block` only stashes the finalized-but-uncommitted block: the
        // full-block ring must NOT be fed here.
        for height in 1..=3 {
            let (block, block_outcome) = block_and_outcome(height);
            indexer.index_block(&block, &block_outcome).await.unwrap();
        }
        assert_eq!(
            indexer.inner.blocks.tip(),
            None,
            "the ring must not be fed before the block is committed"
        );

        // `post_indexing` (post-commit) drains each stash and publishes, in
        // height order. A `None` perps address exercises the full-block feed
        // without an `app_cfg`.
        for height in 1..=3 {
            indexer.publish_committed_block(height, None);
        }

        let got: Vec<u64> = stream.take(3).collect().await;
        assert_eq!(got, vec![1, 2, 3]);
    }
}
