use {
    crate::{BlockFetcher, BlockSource, BlockStore, LiveSubscriber},
    anyhow::bail,
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::{StreamExt, stream::BoxStream},
    std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    tokio::sync::{broadcast, mpsc},
};

/// Genesis floor: block 0 does not exist, so the contiguous prefix and gap
/// detection start at height 1.
const GENESIS_HEIGHT: u64 = 1;

/// Capacity of the channel feeding the coordinator. Bounded so the two writers
/// (backfill + live tail) get backpressure when the coordinator's store writes
/// are the bottleneck.
const COORDINATOR_BUFFER: usize = 1_024;

/// V2 [`BlockSource`]: runs on a node-less host, owns its raw-block store, and
/// pulls blocks from a sentinel. See `design/remote-block-source.md`.
///
/// Two writers feed a single serialized coordinator: the bounded backfill
/// `fetcher` (one gap at a time) and the `subscriber` (the live tail). The
/// coordinator persists each block and advances the contiguous `frontier`,
/// broadcasting newly-contiguous blocks in strict `+1` order — which is what
/// keeps the [`BlockSource`] invariants intact for the projection loop.
pub struct RemoteBlockSource {
    store: Arc<dyn BlockStore>,
    subscriber: Arc<dyn LiveSubscriber>,
    fetcher: Arc<dyn BlockFetcher>,
    /// Highest contiguous height; mutated only by the coordinator task, read
    /// lock-free by `contiguous_frontier`. `0` means "nothing yet".
    frontier: AtomicU64,
    broadcast_tx: broadcast::Sender<Arc<BlockData>>,
}

impl RemoteBlockSource {
    pub fn new(
        store: Arc<dyn BlockStore>,
        subscriber: Arc<dyn LiveSubscriber>,
        fetcher: Arc<dyn BlockFetcher>,
        pubsub_buffer_size: usize,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(pubsub_buffer_size);
        Self {
            store,
            subscriber,
            fetcher,
            frontier: AtomicU64::new(0),
            broadcast_tx,
        }
    }

    /// The single serialized point that owns `frontier` + broadcast. Drains the
    /// coordinator channel, persists each block, and advances the contiguous
    /// prefix.
    ///
    /// Persist-before-advance is the invariant: a height is broadcast (and the
    /// frontier moved onto it) only once it is durable and the whole prefix
    /// below it is present — so `get(h)` always succeeds for `h <= frontier`.
    async fn run_coordinator(
        self: Arc<Self>,
        mut coordinator_rx: mpsc::Receiver<BlockData>,
    ) -> AnyResult<()> {
        while let Some(block) = coordinator_rx.recv().await {
            let height = block.height();
            self.store.put(height, &block).await?;

            // Only an arrival exactly at the prefix edge can extend it. A block
            // ahead of the frontier (the live tail during backfill) is now
            // durable and gets swept up when the prefix reaches it.
            if height == self.frontier.load(Ordering::Acquire) + 1 {
                self.broadcast_tx.send(Arc::new(block)).ok();
                self.frontier.store(height, Ordering::Release);

                // Sweep over any already-stored successors — islands left by a
                // previous run, or the live tail we are now catching up to.
                let mut next_height = height + 1;
                while let Some(next_block) = self.store.get(next_height).await? {
                    self.broadcast_tx.send(Arc::new(next_block)).ok();
                    self.frontier.store(next_height, Ordering::Release);
                    next_height += 1;
                }
            }
        }

        Ok(())
    }

    /// Backfill one gap `[from, to]` through the fetcher, **validating** that
    /// the stream delivers exactly that contiguous range before forwarding each
    /// block to the coordinator. The fetcher is best-effort; this is where the
    /// source enforces correctness (see the [`BlockFetcher`] contract).
    async fn backfill_gap(
        &self,
        from: u64,
        to: u64,
        coordinator_tx: &mpsc::Sender<BlockData>,
    ) -> AnyResult<()> {
        let mut stream = self.fetcher.spawn(from, to);
        let mut expected_height = from;

        while let Some(block) = stream.recv().await {
            let height = block.height();
            if height != expected_height {
                bail!(
                    "fetcher emitted height {height}, expected {expected_height} in gap [{from}, {to}]"
                );
            }
            if coordinator_tx.send(block).await.is_err() {
                // Coordinator gone — the source is shutting down.
                return Ok(());
            }
            expected_height += 1;
        }

        if expected_height <= to {
            bail!(
                "fetch stream for gap [{from}, {to}] ended at {expected_height}, before reaching {to}"
            );
        }

        Ok(())
    }

    /// Drain the live block stream into the coordinator. Runs for the source's
    /// lifetime; an error on the stream ends it (and, in turn, `run`). The
    /// subscriber owns reconnection — a stream that yields an error is fatal.
    async fn drain_live(
        self: Arc<Self>,
        mut live_blocks: BoxStream<'static, AnyResult<BlockData>>,
        coordinator_tx: mpsc::Sender<BlockData>,
    ) -> AnyResult<()> {
        while let Some(block) = live_blocks.next().await {
            if coordinator_tx.send(block?).await.is_err() {
                return Ok(()); // coordinator gone
            }
        }

        Ok(())
    }
}

#[async_trait]
impl BlockSource for RemoteBlockSource {
    async fn run(self: Arc<Self>) -> AnyResult<()> {
        // Resume from our own store: the frontier is the contiguous prefix we
        // already hold.
        if let Some(resume_height) = self.store.max_contiguous(GENESIS_HEIGHT).await? {
            self.frontier.store(resume_height, Ordering::Release);
        }

        // Single serialized coordinator behind a bounded channel.
        let (coordinator_tx, coordinator_rx) = mpsc::channel(COORDINATOR_BUFFER);
        let coordinator_task = tokio::spawn(self.clone().run_coordinator(coordinator_rx));

        // Subscribe: the live tip bounds the backfill; the live tail streams
        // from the tip onward into the coordinator, concurrently.
        let (live_tip, live_blocks) = self.subscriber.subscribe().await?;
        let live_drain_task =
            tokio::spawn(self.clone().drain_live(live_blocks, coordinator_tx.clone()));

        // Backfill every gap below the live tip, lowest-first so the frontier
        // climbs as early as possible. The seam at the tip has no gap:
        // `[1, live_tip)` is the fetcher's, `[live_tip, ∞)` the subscriber's.
        for (from, to) in self.store.gaps(GENESIS_HEIGHT, live_tip).await? {
            self.backfill_gap(from, to, &coordinator_tx).await?;
        }
        // Backfill done; only the live tail keeps feeding the coordinator.
        drop(coordinator_tx);

        // Run until the live tail or the coordinator terminates (error or
        // shutdown); surface whichever error came first.
        let (live_drain_result, coordinator_result) =
            tokio::try_join!(live_drain_task, coordinator_task)?;
        live_drain_result?;
        coordinator_result?;

        Ok(())
    }

    async fn get(&self, height: u64) -> AnyResult<Option<BlockData>> {
        self.store.get(height).await
    }

    fn subscribe(&self) -> broadcast::Receiver<Arc<BlockData>> {
        self.broadcast_tx.subscribe()
    }

    async fn contiguous_frontier(&self) -> AnyResult<Option<u64>> {
        let height = self.frontier.load(Ordering::Acquire);
        Ok((height != 0).then_some(height))
    }
}
