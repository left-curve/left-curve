mod fetcher;
mod islands;
mod store;
mod subscriber;

pub use {
    fetcher::{BlockFetcher, FetchStream, SentinelBlockFetcher, SentinelFetcherConfig},
    store::{BlockStore, MemoryBlockStore},
    subscriber::LiveSubscriber,
};

use {
    self::islands::Islands,
    crate::BlockSource,
    anyhow::{anyhow, bail},
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::{StreamExt, future::select_all},
    std::{
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    },
    tokio::{
        sync::{Notify, broadcast, mpsc},
        time::sleep,
    },
};

/// Genesis floor: block 0 does not exist, so the contiguous prefix and gap
/// detection start at height 1.
const GENESIS_HEIGHT: u64 = 1;

/// Tuning for [`RemoteBlockSource`]. All fields default to the previously
/// hardcoded values; surface them through the source's `remote.*` config so a
/// deployment can bound backfill RAM.
#[derive(Debug, Clone)]
pub struct RemoteBlockSourceConfig {
    /// Broadcast channel capacity — the live-tail fan-out to projections. Holds
    /// up to this many `Arc<BlockData>` when a subscriber lags; at 2 bps,
    /// 10_000 is ~83 min of buffer before a slow projection is `Lagged`.
    pub pubsub_buffer_size: usize,
    /// Capacity of the channel feeding the coordinator. Bounded so the two
    /// writers (backfill + live tail) get backpressure when the coordinator's
    /// store writes are the bottleneck.
    pub coordinator_buffer: usize,
    /// How often the healer re-checks for gaps when no discontinuity signal has
    /// arrived — the safety net for a silently-dropped block that raised no
    /// signal (no reconnect, so `drain_live` saw no jump).
    pub heal_poll_interval: Duration,
    /// After a discontinuity signal, how long the healer waits before computing
    /// gaps, so a block still in flight from an out-of-order live delivery (a
    /// network reorder, e.g. height 101 before 100) lands first and is not
    /// mistaken for a hole. A genuine hole outlasts it and is then filled.
    pub reorder_grace: Duration,
    /// Backoff before re-subscribing after the live stream ends or errors (and
    /// between failed subscribe attempts). A reconnect resumes at the chain
    /// tip; the downtime hole below it is repaired by the healer.
    pub reconnect_backoff: Duration,
}

impl Default for RemoteBlockSourceConfig {
    fn default() -> Self {
        Self {
            pubsub_buffer_size: 10_000,
            coordinator_buffer: 1_024,
            heal_poll_interval: Duration::from_secs(5),
            reorder_grace: Duration::from_millis(250),
            reconnect_backoff: Duration::from_secs(5),
        }
    }
}

/// V2 [`BlockSource`]: runs on a node-less host, owns its raw-block store, and
/// pulls blocks from a sentinel. See `design/remote-block-source.md`.
///
/// Three tasks feed a single serialized coordinator: the `subscriber`'s live
/// tail (`drain_live`), and a continuous `healer` that backfills any gap below
/// the live tip through the bounded `fetcher` — both the initial history and
/// any later hole left by a subscriber reconnect or a dropped block. The
/// coordinator persists each block and advances the contiguous `frontier`,
/// broadcasting newly-contiguous blocks in strict `+1` order — which is what
/// keeps the [`BlockSource`] invariants intact for the projection loop.
pub struct RemoteBlockSource {
    store: Arc<dyn BlockStore>,
    subscriber: Arc<dyn LiveSubscriber>,
    fetcher: Arc<dyn BlockFetcher>,
    config: RemoteBlockSourceConfig,
    /// Highest contiguous height; mutated only by the coordinator task, read
    /// lock-free by `contiguous_frontier`. `0` means "nothing yet".
    frontier: AtomicU64,
    /// Highest height the live subscriber has delivered — the exclusive upper
    /// bound for gap detection (`[frontier + 1, tip)` is the healer's target,
    /// `tip` onward is still the subscriber's). Seeded to the subscription's
    /// tip `L` and advanced by `drain_live`; never decreases.
    tip: AtomicU64,
    /// Wakes the healer when `drain_live` sees a discontinuity in the live
    /// stream (a reconnect at a higher tip, or a skipped height), so the hole
    /// is repaired promptly instead of waiting for the periodic re-check.
    heal_notify: Notify,
    broadcast_tx: broadcast::Sender<Arc<BlockData>>,
}

impl RemoteBlockSource {
    pub fn new(
        store: Arc<dyn BlockStore>,
        subscriber: Arc<dyn LiveSubscriber>,
        fetcher: Arc<dyn BlockFetcher>,
        config: RemoteBlockSourceConfig,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(config.pubsub_buffer_size);
        Self {
            store,
            subscriber,
            fetcher,
            config,
            frontier: AtomicU64::new(0),
            tip: AtomicU64::new(0),
            heal_notify: Notify::new(),
            broadcast_tx,
        }
    }

    /// The single serialized point that owns `frontier` + broadcast. Drains the
    /// coordinator channel, persists each block, and advances the contiguous
    /// prefix.
    ///
    /// Ordering for a block that extends the prefix (`height == frontier + 1`):
    /// **broadcast → persist → advance the frontier**. The broadcast goes
    /// out first so live projections do not wait on the store write; the
    /// frontier is advanced last, *after* the block is durable, so it never
    /// claims a height that `get(h)` cannot yet serve — `h <= frontier ⟹
    /// get(h) = Some` still holds. A crash in the broadcast→persist window
    /// self-heals: the un-stored height is re-fetched as a gap on restart, and
    /// any projection that already consumed it skips the replay by cursor.
    ///
    /// A block ahead of the prefix (the live tail during backfill, or a reorder)
    /// is an **island**: persisted, with its height remembered in `islands` (a
    /// compact start→end range map). When the prefix edge reaches an island it
    /// is crossed in **one step** — the frontier jumps to the island's top and
    /// only that top is broadcast, so a large catch-up backlog cannot flood the
    /// pubsub; projections fall back to Phase-1 `get()` for the skipped heights.
    /// The map is seeded at boot from the store, so islands left by a previous
    /// run are crossed the same way. Tracking islands in memory is also what
    /// lets the common edge advance with **no store probe** — the wasted
    /// `get(frontier + 1) -> None` per block is gone.
    ///
    /// A store error is intentionally **fatal**: the store is the durability
    /// anchor, so on a write failure the source halts (the process is expected
    /// to restart, re-seeding the frontier from `max_contiguous` and
    /// re-fetching) rather than limping on with in-place retries.
    async fn run_coordinator(
        self: Arc<Self>,
        mut coordinator_rx: mpsc::Receiver<BlockData>,
    ) -> AnyResult<()> {
        // Seed the island map from blocks a previous run left above our resume
        // frontier, as ranges (one entry per stored stretch — cheap).
        let mut islands = {
            let frontier = self.frontier.load(Ordering::Acquire);
            match self.store.max_height().await? {
                Some(max_height) if max_height > frontier => {
                    let gaps = self.store.gaps(frontier + 1, max_height + 1).await?;
                    Islands::from_gaps(frontier + 1, max_height, &gaps)
                },
                _ => Islands::default(),
            }
        };

        while let Some(block) = coordinator_rx.recv().await {
            let height = block.height();
            let frontier = self.frontier.load(Ordering::Acquire);

            // Below the prefix: a duplicate/replay. Persist (idempotent) and
            // ignore.
            if height <= frontier {
                self.store.put(height, &block).await?;
                continue;
            }

            // Ahead of the prefix: an island. Persist and remember it; it is
            // broadcast when the edge reaches it.
            if height > frontier + 1 {
                self.store.put(height, &block).await?;
                islands.insert(height);
                continue;
            }

            // The edge block. Broadcast first (live projections need not wait on
            // the store write), then persist, then advance the frontier.
            let block = Arc::new(block);
            self.broadcast_tx.send(block.clone()).ok();
            self.store.put(height, &block).await?;
            self.frontier.store(height, Ordering::Release);

            // Cross any islands now contiguous with the edge. Each is crossed in
            // one step: jump the frontier to its top, broadcast only that top.
            let mut edge = height;
            while let Some(end) = islands.take_starting_at(edge + 1) {
                let top = self
                    .store
                    .get(end)
                    .await?
                    .ok_or_else(|| anyhow!("island top {end} missing from store"))?;
                self.broadcast_tx.send(Arc::new(top)).ok();
                self.frontier.store(end, Ordering::Release);
                edge = end;
            }
        }

        Ok(())
    }

    /// Continuously repair the contiguous prefix: find any gap below the live
    /// `tip` and backfill it through the fetcher, lowest-first so the frontier
    /// climbs as early as possible.
    ///
    /// This one loop subsumes both the startup backfill (on a fresh or lagging
    /// start the gap is `[GENESIS, tip)`) and the steady-state repair of a hole
    /// left by a subscriber reconnect or a dropped block — the same machinery
    /// for both. The **store is the source of truth**: every iteration
    /// recomputes the gaps from it, so a transient miss simply heals on the
    /// next pass, and a reordered-but-present block is never mistaken for a gap.
    async fn run_healer(self: Arc<Self>, coordinator_tx: mpsc::Sender<BlockData>) -> AnyResult<()> {
        loop {
            // Coordinator gone ⇒ the source is tearing down; stop before doing
            // any work (and before a `send` that would only fail).
            if coordinator_tx.is_closed() {
                return Ok(());
            }

            let frontier = self.frontier.load(Ordering::Acquire);
            let tip = self.tip.load(Ordering::Acquire);

            // Below `frontier` everything is contiguous by construction, so
            // holes can only sit in `[frontier + 1, tip)`. In steady state that
            // window is empty and this is near-free; `tip` itself is excluded
            // because it is the subscriber's, possibly still in flight.
            let gaps = if tip > frontier {
                self.store.gaps(frontier + 1, tip).await?
            } else {
                Vec::new()
            };

            if gaps.is_empty() {
                // Nothing to fill. Sleep until a discontinuity signal, with a
                // periodic re-check as the safety net for a silently-dropped
                // block that raised no signal.
                tokio::select! {
                    _ = self.heal_notify.notified() => {
                        // Absorb an in-flight reorder before deciding there is a
                        // gap, so a transient out-of-order delivery does not
                        // trigger a redundant fetch. A real hole outlasts this.
                        sleep(self.config.reorder_grace).await;
                    }
                    _ = sleep(self.config.heal_poll_interval) => {}
                }
                continue;
            }

            for (from, to) in gaps {
                #[cfg(feature = "tracing")]
                tracing::info!(from, to, "healer backfilling gap");
                self.backfill_gap(from, to, &coordinator_tx).await?;
            }
        }
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

    /// Follow the live tail, reconnecting on every drop. Owns the subscription
    /// lifecycle: (re)subscribe, drain blocks into the coordinator (tracking the
    /// tip and flagging discontinuities), and on a stream end/error back off and
    /// re-subscribe. A reconnect resumes at the chain tip; the downtime hole
    /// below it is repaired by the healer like any other gap — which is why a
    /// dropped stream no longer takes the source down. Runs for the source's
    /// lifetime; returns only when the coordinator is gone.
    async fn drain_live(self: Arc<Self>, coordinator_tx: mpsc::Sender<BlockData>) -> AnyResult<()> {
        loop {
            if coordinator_tx.is_closed() {
                return Ok(()); // coordinator gone — source shutting down
            }

            let (live_tip, mut live_blocks) = match self.subscriber.subscribe().await {
                Ok(subscription) => subscription,
                Err(_error) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(error = %_error, "live subscribe failed; retrying");
                    sleep(self.config.reconnect_backoff).await;
                    continue;
                },
            };

            // A (re)subscribe resumes at the new tip: raise it and wake the
            // healer to fill whatever gap now sits below it — the initial
            // history on first connect, a downtime hole on a reconnect.
            self.tip.fetch_max(live_tip, Ordering::AcqRel);
            self.heal_notify.notify_one();

            loop {
                match live_blocks.next().await {
                    Some(Ok(block)) => {
                        let height = block.height();

                        // `fetch_max` advances the tip and hands back the
                        // previous value; a delivered height beyond `prev + 1`
                        // means blocks went missing (a skip, or a reconnect at a
                        // higher tip), so wake the healer. (A reorder also trips
                        // this; the healer's grace absorbs it — see `run_healer`.)
                        let prev_tip = self.tip.fetch_max(height, Ordering::AcqRel);
                        if height > prev_tip + 1 {
                            self.heal_notify.notify_one();
                        }

                        if coordinator_tx.send(block).await.is_err() {
                            return Ok(()); // coordinator gone
                        }
                    },
                    Some(Err(_error)) => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(error = %_error, "live stream error; reconnecting");
                        break;
                    },
                    None => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("live stream ended; reconnecting");
                        break;
                    },
                }
            }

            sleep(self.config.reconnect_backoff).await;
        }
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

        // Single serialized coordinator behind a bounded channel, fed by the
        // live tail (`drain_live`, which owns its own subscription lifecycle)
        // and the healer's backfill.
        let (coordinator_tx, coordinator_rx) = mpsc::channel(self.config.coordinator_buffer);

        let coordinator = tokio::spawn(self.clone().run_coordinator(coordinator_rx));
        let drain = tokio::spawn(self.clone().drain_live(coordinator_tx.clone()));
        let healer = tokio::spawn(self.clone().run_healer(coordinator_tx));

        // All three run for the source's lifetime. Whichever returns first (a
        // clean end or an error) tears the others down — no detached task
        // outlives `run`.
        let (result, _index, remaining) = select_all([coordinator, drain, healer]).await;
        for handle in remaining {
            handle.abort();
        }

        match result {
            Ok(task_result) => task_result,
            Err(join_err) => Err(anyhow!("block source task panicked: {join_err}")),
        }
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

// ---- tests ----

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_primitives::{Block, BlockInfo, BlockOutcome, Hash256, Timestamp},
        futures::stream::{self, BoxStream},
        tokio::time::timeout,
    };

    /// A minimal `BlockData` carrying only the height the coordinator/healer
    /// logic reads — empty txs/outcomes, zero hashes.
    fn block(height: u64) -> BlockData {
        BlockData {
            block: Block {
                info: BlockInfo {
                    height,
                    timestamp: Timestamp::from_nanos(0),
                    hash: Hash256::ZERO,
                },
                txs: vec![],
            },
            outcome: BlockOutcome {
                height,
                app_hash: Hash256::ZERO,
                cron_outcomes: vec![],
                tx_outcomes: vec![],
            },
        }
    }

    /// A fetcher for which every requested height "exists": `spawn(from, to)`
    /// streams exactly `[from, to]` ascending.
    struct MockFetcher;

    impl BlockFetcher for MockFetcher {
        fn spawn(&self, from: u64, to: u64) -> FetchStream {
            let (tx, rx) = mpsc::channel(1_024);
            let handle = tokio::spawn(async move {
                for height in from..=to {
                    if tx.send(block(height)).await.is_err() {
                        return;
                    }
                }
            });
            FetchStream::new(rx, handle)
        }
    }

    /// A subscriber that yields a scripted height sequence (to simulate skips
    /// and reconnects), then pends forever so the source keeps running.
    struct MockSubscriber {
        live_tip: u64,
        script: std::sync::Mutex<Option<Vec<u64>>>,
    }

    impl MockSubscriber {
        fn new(live_tip: u64, script: Vec<u64>) -> Self {
            Self {
                live_tip,
                script: std::sync::Mutex::new(Some(script)),
            }
        }
    }

    #[async_trait]
    impl LiveSubscriber for MockSubscriber {
        async fn subscribe(&self) -> AnyResult<(u64, BoxStream<'static, AnyResult<BlockData>>)> {
            let script = self.script.lock().unwrap().take().unwrap_or_default();
            let scripted = stream::iter(script.into_iter().map(|h| Ok(block(h))));
            let stream = scripted.chain(stream::pending::<AnyResult<BlockData>>());
            Ok((self.live_tip, Box::pin(stream)))
        }
    }

    /// A subscriber that yields a queue of scripted episodes — each a tip, a
    /// height sequence, and whether the stream pends (`true`) or ends (`false`)
    /// after it. An episode that ends makes `drain_live` reconnect to the next.
    struct ScriptedSubscriber {
        episodes: std::sync::Mutex<std::collections::VecDeque<(u64, Vec<u64>, bool)>>,
    }

    impl ScriptedSubscriber {
        fn new(episodes: Vec<(u64, Vec<u64>, bool)>) -> Self {
            Self {
                episodes: std::sync::Mutex::new(episodes.into()),
            }
        }
    }

    #[async_trait]
    impl LiveSubscriber for ScriptedSubscriber {
        async fn subscribe(&self) -> AnyResult<(u64, BoxStream<'static, AnyResult<BlockData>>)> {
            let (tip, heights, pend) = match self.episodes.lock().unwrap().pop_front() {
                Some(episode) => episode,
                None => bail!("scripted subscriber exhausted"),
            };
            let scripted = stream::iter(heights.into_iter().map(|h| Ok(block(h))));
            let stream: BoxStream<'static, AnyResult<BlockData>> = if pend {
                Box::pin(scripted.chain(stream::pending::<AnyResult<BlockData>>()))
            } else {
                Box::pin(scripted)
            };
            Ok((tip, stream))
        }
    }

    /// A store whose `put` blocks until `release` is signalled, so a test can
    /// observe the broadcast firing before the block is persisted.
    #[derive(Default)]
    struct GatingStore {
        inner: MemoryBlockStore,
        gate: Notify,
    }

    impl GatingStore {
        fn release(&self) {
            self.gate.notify_one();
        }
    }

    #[async_trait]
    impl BlockStore for GatingStore {
        async fn put(&self, height: u64, data: &BlockData) -> AnyResult<()> {
            self.gate.notified().await;
            self.inner.put(height, data).await
        }

        async fn get(&self, height: u64) -> AnyResult<Option<BlockData>> {
            self.inner.get(height).await
        }

        async fn max_contiguous(&self, floor: u64) -> AnyResult<Option<u64>> {
            self.inner.max_contiguous(floor).await
        }

        async fn max_height(&self) -> AnyResult<Option<u64>> {
            self.inner.max_height().await
        }

        async fn gaps(&self, from: u64, to: u64) -> AnyResult<Vec<(u64, u64)>> {
            self.inner.gaps(from, to).await
        }
    }

    fn source_with(store: Arc<MemoryBlockStore>) -> Arc<RemoteBlockSource> {
        Arc::new(RemoteBlockSource::new(
            store,
            Arc::new(MockSubscriber::new(0, vec![])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ))
    }

    /// Read the next `count` broadcast heights, asserting they are `start..`.
    async fn assert_broadcast(
        rx: &mut broadcast::Receiver<Arc<BlockData>>,
        start: u64,
        count: u64,
    ) {
        for expected in start..start + count {
            let block = timeout(Duration::from_secs(5), rx.recv())
                .await
                .expect("timed out waiting for broadcast")
                .expect("broadcast closed");
            assert_eq!(block.height(), expected);
        }
    }

    #[tokio::test]
    async fn coordinator_broadcasts_contiguous() {
        let source = source_with(Arc::new(MemoryBlockStore::default()));
        let mut rx = source.subscribe();

        let (tx, coordinator_rx) = mpsc::channel(64);
        let coordinator = tokio::spawn(source.clone().run_coordinator(coordinator_rx));

        for height in 1..=3 {
            tx.send(block(height)).await.unwrap();
        }
        drop(tx);
        coordinator.await.unwrap().unwrap();

        assert_broadcast(&mut rx, 1, 3).await;
        assert_eq!(source.frontier.load(Ordering::Acquire), 3);
    }

    #[tokio::test]
    async fn coordinator_handles_out_of_order() {
        let source = source_with(Arc::new(MemoryBlockStore::default()));
        let mut rx = source.subscribe();

        let (tx, coordinator_rx) = mpsc::channel(64);
        let coordinator = tokio::spawn(source.clone().run_coordinator(coordinator_rx));

        // 2 arrives before 1: it waits as an island, then 1 unlocks both — the
        // broadcast is still strictly +1 (1, then 2).
        tx.send(block(2)).await.unwrap();
        tx.send(block(1)).await.unwrap();
        drop(tx);
        coordinator.await.unwrap().unwrap();

        assert_broadcast(&mut rx, 1, 2).await;
        assert_eq!(source.frontier.load(Ordering::Acquire), 2);
    }

    #[tokio::test]
    async fn coordinator_bulk_advances_across_island() {
        let store = Arc::new(MemoryBlockStore::default());
        // A multi-block island left by a previous run.
        for height in 3..=5 {
            store.put(height, &block(height)).await.unwrap();
        }

        let source = source_with(store.clone());
        let mut rx = source.subscribe();

        let (tx, coordinator_rx) = mpsc::channel(64);
        let coordinator = tokio::spawn(source.clone().run_coordinator(coordinator_rx));

        tx.send(block(1)).await.unwrap();
        tx.send(block(2)).await.unwrap();
        drop(tx);
        coordinator.await.unwrap().unwrap();

        // Edges 1, 2 are pushed; the island [3, 5] is bulk-advanced, so only its
        // top (5) is broadcast — 3 and 4 are not (projections pull them via
        // `get()`). The frontier still reaches 5.
        assert_eq!(rx.recv().await.unwrap().height(), 1);
        assert_eq!(rx.recv().await.unwrap().height(), 2);
        assert_eq!(rx.recv().await.unwrap().height(), 5);
        assert!(rx.try_recv().is_err());
        assert_eq!(source.frontier.load(Ordering::Acquire), 5);
        assert!(store.get(3).await.unwrap().is_some());
        assert!(store.get(4).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn healer_fills_skipped_block() {
        let store = Arc::new(MemoryBlockStore::default());
        // Resume point: 1..=9 already contiguous.
        for height in 1..=9 {
            store.put(height, &block(height)).await.unwrap();
        }

        // Subscriber: tip 10, delivers 10, 11, 13 (skips 12), then pends.
        let source = Arc::new(RemoteBlockSource::new(
            store,
            Arc::new(MockSubscriber::new(10, vec![10, 11, 13])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ));
        let mut rx = source.subscribe();

        let run = tokio::spawn(source.clone().run());

        // The hole at 12 is healed; the broadcast is contiguous 10..=13.
        assert_broadcast(&mut rx, 10, 4).await;
        assert_eq!(source.frontier.load(Ordering::Acquire), 13);

        run.abort();
    }

    #[tokio::test]
    async fn healer_fills_reconnect_hole() {
        let store = Arc::new(MemoryBlockStore::default());
        for height in 1..=9 {
            store.put(height, &block(height)).await.unwrap();
        }

        // Subscriber: tip 10, delivers 10, 11, then "reconnects" at 20 (the
        // 12..=19 downtime hole), then pends.
        let source = Arc::new(RemoteBlockSource::new(
            store,
            Arc::new(MockSubscriber::new(10, vec![10, 11, 20])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ));
        let mut rx = source.subscribe();

        let run = tokio::spawn(source.clone().run());

        // The 12..=19 hole is healed; the broadcast is contiguous 10..=20.
        assert_broadcast(&mut rx, 10, 11).await;
        assert_eq!(source.frontier.load(Ordering::Acquire), 20);

        run.abort();
    }

    #[tokio::test]
    async fn contiguous_frontier_none_until_advanced() {
        let source = source_with(Arc::new(MemoryBlockStore::default()));
        assert_eq!(source.contiguous_frontier().await.unwrap(), None);
        source.frontier.store(7, Ordering::Release);
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(7));
    }

    #[tokio::test]
    async fn broadcast_precedes_store_for_edge_block() {
        // The store write is gated, so we can observe the order.
        let store = Arc::new(GatingStore::default());
        let source = Arc::new(RemoteBlockSource::new(
            store.clone(),
            Arc::new(MockSubscriber::new(0, vec![])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ));
        let mut rx = source.subscribe();

        let (tx, coordinator_rx) = mpsc::channel(64);
        let coordinator = tokio::spawn(source.clone().run_coordinator(coordinator_rx));

        // Block 1 is at the prefix edge: the coordinator broadcasts it, then
        // blocks on the gated store write.
        tx.send(block(1)).await.unwrap();

        // The broadcast arrives even though the store write has not run...
        let broadcast = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("broadcast should fire before the store write")
            .expect("broadcast closed");
        assert_eq!(broadcast.height(), 1);
        // ...the block is not persisted yet, and the frontier stays behind it.
        assert_eq!(store.get(1).await.unwrap(), None);
        assert_eq!(source.frontier.load(Ordering::Acquire), 0);

        // Release the write; persistence completes, then the frontier advances.
        store.release();
        for _ in 0..200 {
            if source.frontier.load(Ordering::Acquire) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(source.frontier.load(Ordering::Acquire), 1);
        assert_eq!(store.get(1).await.unwrap().unwrap().height(), 1);

        drop(tx);
        coordinator.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn drain_live_reconnects_after_stream_end() {
        let store = Arc::new(MemoryBlockStore::default());
        for height in 1..=9 {
            store.put(height, &block(height)).await.unwrap();
        }

        // Episode 1: tip 10, deliver 10, 11, then the stream ENDS (connection
        // drop). Episode 2: tip 20 (12..=19 produced during the downtime),
        // deliver 20, 21, then pend.
        let subscriber = Arc::new(ScriptedSubscriber::new(vec![
            (10, vec![10, 11], false),
            (20, vec![20], true),
        ]));
        let config = RemoteBlockSourceConfig {
            reconnect_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let source = Arc::new(RemoteBlockSource::new(
            store,
            subscriber,
            Arc::new(MockFetcher),
            config,
        ));
        let mut rx = source.subscribe();

        let run = tokio::spawn(source.clone().run());

        // 10, 11 from episode 1; the stream drops, the source reconnects, the
        // healer fills the 12..=19 downtime hole, and 20 arrives — the broadcast
        // is contiguous 10..=20.
        assert_broadcast(&mut rx, 10, 11).await;
        assert_eq!(source.frontier.load(Ordering::Acquire), 20);

        run.abort();
    }
}
