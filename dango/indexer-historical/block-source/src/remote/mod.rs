mod fetcher;
mod store;
mod subscriber;

pub use {
    fetcher::{
        BlockFetcher, BlockRangeClient, FetchStream, MAX_BLOCK_RANGE, SentinelBlockFetcher,
        SentinelFetcherConfig, SentinelRangeClient,
    },
    store::{BlockStore, GENESIS_HEIGHT, MemoryBlockStore, RocksdbBlockStore},
    subscriber::{FullBlockSubscriber, LiveSubscriber},
};

use {
    crate::BlockSource,
    anyhow::{anyhow, bail},
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::{StreamExt, future::select_all},
    std::{sync::Arc, time::Duration},
    tokio::{
        sync::{Notify, broadcast, mpsc},
        time::sleep,
    },
};

/// Tuning for [`RemoteBlockSource`]. All fields default to the previously
/// hardcoded values; surface them through the source's `remote.*` config so a
/// deployment can bound backfill RAM.
#[derive(Debug, Clone)]
pub struct RemoteBlockSourceConfig {
    /// Broadcast channel capacity — the live-tail fan-out to projections. A
    /// tokio broadcast is a **ring**: it keeps the most-recent `capacity`
    /// `Arc<BlockData>` resident at all times, so this is a RAM knob, not a time
    /// one. At the measured mainnet payloads (median ~20 KB, p90 ~150 KB borsh)
    /// 2_000 is ~40 MB typical / ~300 MB peak; a lagging projection is caught by
    /// the Phase-1 `get()` recovery, so a larger ring buys little.
    pub pubsub_buffer_size: usize,
    /// Capacity of the channel feeding the coordinator. Bounded so the two
    /// writers (backfill + live tail) get backpressure when the coordinator's
    /// store writes are the bottleneck.
    pub coordinator_buffer: usize,
    /// How often the healer re-checks for gaps when no discontinuity signal has
    /// arrived — the safety net for a silently-dropped block that raised no
    /// signal (no reconnect, so `drain_live` saw no jump).
    pub heal_poll_interval: Duration,
    /// After a discontinuity signal, how long the healer waits before checking
    /// for a gap, so a block still in flight from an out-of-order live delivery
    /// (a network reorder, e.g. height 101 before 100) lands first and is not
    /// mistaken for a hole. A genuine hole outlasts it and is then filled.
    pub reorder_grace: Duration,
    /// Backoff before re-subscribing after the live stream ends or errors (and
    /// between failed subscribe attempts). A reconnect resumes at the chain
    /// tip; the downtime hole below it is repaired by the healer.
    pub reconnect_backoff: Duration,
    /// Backoff after a gap backfill fails its contiguity check (the fetcher
    /// delivered a wrong height or ended early) before the healer retries that
    /// gap. Keeps a misbehaving fetcher from spinning the healer in a tight
    /// loop, while keeping the failure non-fatal to the source.
    pub heal_retry_backoff: Duration,
}

impl Default for RemoteBlockSourceConfig {
    fn default() -> Self {
        Self {
            pubsub_buffer_size: 2_000,
            coordinator_buffer: 1_024,
            heal_poll_interval: Duration::from_secs(5),
            reorder_grace: Duration::from_millis(250),
            reconnect_backoff: Duration::from_secs(5),
            heal_retry_backoff: Duration::from_secs(1),
        }
    }
}

/// V2 [`BlockSource`]: runs on a node-less host, owns its raw-block store, and
/// pulls blocks from a sentinel. See `design/remote-block-source.md`.
///
/// Two tasks feed a single serialized coordinator: the `subscriber`'s live tail
/// (`drain_live`), and a continuous `healer` that backfills any gap the store
/// reports through the bounded `fetcher` — both the initial history and any
/// later hole left by a reconnect or a dropped block. The **store owns the
/// topology**: its `put` persists each block and reports when the contiguous
/// frontier advances, so the coordinator does nothing but forward that top to
/// the broadcast. Broadcasts are strictly ascending (`+1` on the live tail, a
/// skip-to-top on a bulk-advance), which is what keeps the [`BlockSource`]
/// invariants intact for the projection loop.
pub struct RemoteBlockSource {
    store: Arc<dyn BlockStore>,
    subscriber: Arc<dyn LiveSubscriber>,
    fetcher: Arc<dyn BlockFetcher>,
    config: RemoteBlockSourceConfig,
    /// Wakes the healer when the live tail shows a discontinuity (a reconnect
    /// at a higher tip, or a skipped height), so a hole is repaired promptly
    /// rather than waiting for the periodic re-check.
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
            heal_notify: Notify::new(),
            broadcast_tx,
        }
    }

    /// The single serialized writer: drain the coordinator channel, hand each
    /// block to the store, and broadcast whenever the store reports the
    /// frontier advanced.
    ///
    /// All topology lives in the store now: `put` decides whether the block is
    /// a duplicate, an island above a gap, the next edge, or the bridge that
    /// crosses a stored backlog — and returns the new contiguous frontier only
    /// in the last two cases. We broadcast just that top: on a normal `+1` it
    /// *is* the block we hold; on a bulk-advance the skipped heights are already
    /// durable and projections pull them via `get()`, so a large catch-up
    /// backlog never floods the pubsub.
    ///
    /// Order is **persist → broadcast**: the block is durable before any
    /// projection hears of it, so `h <= frontier ⟹ get(h) = Some` holds. A
    /// store error is intentionally **fatal** — the store is the durability
    /// anchor, so on a write failure the source halts (the process restarts and
    /// resumes from the store) rather than limping on.
    async fn run_coordinator(
        self: Arc<Self>,
        mut coordinator_rx: mpsc::Receiver<BlockData>,
    ) -> AnyResult<()> {
        while let Some(block) = coordinator_rx.recv().await {
            let height = block.height();

            let Some(frontier) = self.store.put(height, &block).await? else {
                // Duplicate, or an island above a gap — nothing newly
                // contiguous to broadcast.
                continue;
            };

            // The frontier advanced to `frontier`. Broadcast only its top: the
            // block we just put when it is a plain `+1`, otherwise the stored
            // top of the run we just bridged.
            let top = if frontier == height {
                block
            } else {
                self.store
                    .get(frontier)
                    .await?
                    .ok_or_else(|| anyhow!("frontier {frontier} missing from store"))?
            };
            self.broadcast_tx.send(Arc::new(top)).ok();
        }

        Ok(())
    }

    /// Continuously repair the contiguous prefix: ask the store for its lowest
    /// gap and backfill it through the fetcher, so the frontier climbs
    /// lowest-first.
    ///
    /// One loop subsumes both the startup backfill (on a fresh start the gap is
    /// the whole history below the live tail) and the steady-state repair of a
    /// hole left by a reconnect or a dropped block. The **store is the source
    /// of truth**: each pass re-reads its lowest gap, so a transient miss heals
    /// on the next pass and a reordered-but-present block is never mistaken for
    /// a hole.
    async fn run_healer(self: Arc<Self>, coordinator_tx: mpsc::Sender<BlockData>) -> AnyResult<()> {
        loop {
            // Coordinator gone ⇒ the source is tearing down; stop before doing
            // any work (and before a `send` that would only fail).
            if coordinator_tx.is_closed() {
                return Ok(());
            }

            match self.store.lowest_gap().await? {
                Some((from, to)) => {
                    #[cfg(feature = "tracing")]
                    tracing::info!(from, to, "healer backfilling gap");
                    // A fetcher that violates the contiguity contract (wrong
                    // height, or a stream that ends before `to`) fails this one
                    // gap — but it must not take the whole source (and every
                    // projection) down. Log, back off, and retry the gap on the
                    // next pass; a *persistent* failure stays visible as a frozen
                    // frontier (known issue #5), not a crash loop. Genuine
                    // store-write errors remain fatal (they surface from the
                    // coordinator, not here).
                    if let Err(_err) = self.backfill_gap(from, to, &coordinator_tx).await {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(
                            from,
                            to,
                            error = %_err,
                            "gap backfill failed; retrying after backoff"
                        );
                        sleep(self.config.heal_retry_backoff).await;
                    }
                },
                None => {
                    // Gap-free. Sleep until a discontinuity signal, with a
                    // periodic re-check as the safety net for a silently-dropped
                    // block that raised no signal. After a signal, a short grace
                    // lets an out-of-order live delivery land before we re-check,
                    // so a transient reorder does not trigger a redundant fetch.
                    tokio::select! {
                        _ = self.heal_notify.notified() => {
                            sleep(self.config.reorder_grace).await;
                        }
                        _ = sleep(self.config.heal_poll_interval) => {}
                    }
                },
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
    /// lifecycle: (re)subscribe, drain blocks into the coordinator (flagging
    /// discontinuities so the healer repairs them), and on a stream end/error
    /// back off and re-subscribe. A reconnect resumes at the chain tip; the
    /// downtime hole below it is repaired by the healer like any other gap —
    /// which is why a dropped stream no longer takes the source down. Runs for
    /// the source's lifetime; returns only when the coordinator is gone.
    async fn drain_live(self: Arc<Self>, coordinator_tx: mpsc::Sender<BlockData>) -> AnyResult<()> {
        loop {
            if coordinator_tx.is_closed() {
                return Ok(()); // coordinator gone — source shutting down
            }

            // Resume the feed just above the contiguous frontier, so a reconnect
            // replays the downtime hole with no gap; `None` (fresh, empty store)
            // starts at the live tip.
            let since = self
                .store
                .contiguous_frontier()
                .await?
                .map(|frontier| frontier + 1);
            let mut live_blocks = match self.subscriber.subscribe(since).await {
                Ok(stream) => stream,
                Err(_error) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(error = %_error, "live subscribe failed; retrying");
                    sleep(self.config.reconnect_backoff).await;
                    continue;
                },
            };

            // A (re)subscribe resumes at the new tip: wake the healer to fill
            // whatever gap now sits below it — the initial history on first
            // connect, a downtime hole on a reconnect.
            self.heal_notify.notify_one();

            // Highest height delivered so far this subscription. `None` until the
            // first block arrives, which sets the baseline; after that a jump
            // beyond `prev + 1` means blocks went missing.
            let mut prev: Option<u64> = None;

            loop {
                match live_blocks.next().await {
                    Some(Ok(block)) => {
                        let height = block.height();

                        match prev {
                            // First block of this subscription: just the baseline.
                            None => prev = Some(height),
                            // A skip (or a reconnect at a higher tip, or a
                            // reorder) leaves a hole; wake the healer. The grace
                            // in `run_healer` absorbs a transient reorder.
                            Some(p) => {
                                if height > p + 1 {
                                    self.heal_notify.notify_one();
                                }
                                prev = Some(height.max(p));
                            },
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
        // Nothing to seed: the store owns the contiguous frontier and derives it
        // from the heights it already holds.
        //
        // A single serialized coordinator behind a bounded channel, fed by the
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
        self.store.contiguous_frontier().await
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
        script: std::sync::Mutex<Option<Vec<u64>>>,
    }

    impl MockSubscriber {
        fn new(script: Vec<u64>) -> Self {
            Self {
                script: std::sync::Mutex::new(Some(script)),
            }
        }
    }

    #[async_trait]
    impl LiveSubscriber for MockSubscriber {
        async fn subscribe(
            &self,
            _since: Option<u64>,
        ) -> AnyResult<BoxStream<'static, AnyResult<BlockData>>> {
            let script = self.script.lock().unwrap().take().unwrap_or_default();
            let scripted = stream::iter(script.into_iter().map(|h| Ok(block(h))));
            let stream = scripted.chain(stream::pending::<AnyResult<BlockData>>());
            Ok(Box::pin(stream))
        }
    }

    /// A subscriber that yields a queue of scripted episodes — each a height
    /// sequence and whether the stream pends (`true`) or ends (`false`) after
    /// it. An episode that ends makes `drain_live` reconnect to the next.
    struct ScriptedSubscriber {
        episodes: std::sync::Mutex<std::collections::VecDeque<(Vec<u64>, bool)>>,
    }

    impl ScriptedSubscriber {
        fn new(episodes: Vec<(Vec<u64>, bool)>) -> Self {
            Self {
                episodes: std::sync::Mutex::new(episodes.into()),
            }
        }
    }

    #[async_trait]
    impl LiveSubscriber for ScriptedSubscriber {
        async fn subscribe(
            &self,
            _since: Option<u64>,
        ) -> AnyResult<BoxStream<'static, AnyResult<BlockData>>> {
            let (heights, pend) = match self.episodes.lock().unwrap().pop_front() {
                Some(episode) => episode,
                None => bail!("scripted subscriber exhausted"),
            };
            let scripted = stream::iter(heights.into_iter().map(|h| Ok(block(h))));
            let stream: BoxStream<'static, AnyResult<BlockData>> = if pend {
                Box::pin(scripted.chain(stream::pending::<AnyResult<BlockData>>()))
            } else {
                Box::pin(scripted)
            };
            Ok(Box::pin(stream))
        }
    }

    fn source_with(store: Arc<MemoryBlockStore>) -> Arc<RemoteBlockSource> {
        Arc::new(RemoteBlockSource::new(
            store,
            Arc::new(MockSubscriber::new(vec![])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ))
    }

    /// Next broadcast height, with a timeout so a missed wake-up fails the test
    /// instead of hanging.
    async fn recv_height(rx: &mut broadcast::Receiver<Arc<BlockData>>) -> u64 {
        timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for broadcast")
            .expect("broadcast closed")
            .height()
    }

    /// Drain broadcasts up to and including `target`, asserting the sequence is
    /// strictly ascending (the broadcast invariant: `+1` or a skip-to-top, never
    /// out of order or duplicated). Returns the heights seen.
    async fn drain_until(rx: &mut broadcast::Receiver<Arc<BlockData>>, target: u64) -> Vec<u64> {
        let mut seen = Vec::new();
        loop {
            let height = recv_height(rx).await;
            if let Some(&last) = seen.last() {
                assert!(
                    height > last,
                    "broadcast not ascending: {last} then {height}"
                );
            }
            seen.push(height);
            if height == target {
                return seen;
            }
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

        for height in 1..=3 {
            assert_eq!(recv_height(&mut rx).await, height);
        }
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(3));
    }

    #[tokio::test]
    async fn coordinator_skips_to_top_on_out_of_order() {
        let store = Arc::new(MemoryBlockStore::default());
        let source = source_with(store.clone());
        let mut rx = source.subscribe();

        let (tx, coordinator_rx) = mpsc::channel(64);
        let coordinator = tokio::spawn(source.clone().run_coordinator(coordinator_rx));

        // 2 waits above the gap at 1; when 1 lands the frontier jumps to 2, so
        // only the top (2) is broadcast — 1 is durable and pulled via get().
        tx.send(block(2)).await.unwrap();
        tx.send(block(1)).await.unwrap();
        drop(tx);
        coordinator.await.unwrap().unwrap();

        assert_eq!(recv_height(&mut rx).await, 2);
        assert!(rx.try_recv().is_err());
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(2));
        assert!(store.get(1).await.unwrap().is_some());
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

        // 1 advances the frontier to 1 (broadcast). 2 bridges to the [3, 5]
        // island, jumping the frontier to 5 — only the top (5) is broadcast;
        // 2, 3, 4 are durable and pulled via get().
        assert_eq!(recv_height(&mut rx).await, 1);
        assert_eq!(recv_height(&mut rx).await, 5);
        assert!(rx.try_recv().is_err());
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(5));
        for height in 2..=4 {
            assert!(store.get(height).await.unwrap().is_some());
        }
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
            store.clone(),
            Arc::new(MockSubscriber::new(vec![10, 11, 13])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ));
        let mut rx = source.subscribe();

        let run = tokio::spawn(source.clone().run());

        // 10, 11 push through; 13 waits above the hole at 12; the healer fills
        // 12, the frontier jumps to 13, and only the top (13) is broadcast.
        assert_eq!(recv_height(&mut rx).await, 10);
        assert_eq!(recv_height(&mut rx).await, 11);
        assert_eq!(recv_height(&mut rx).await, 13);
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(13));
        assert!(store.get(12).await.unwrap().is_some());

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
            store.clone(),
            Arc::new(MockSubscriber::new(vec![10, 11, 20])),
            Arc::new(MockFetcher),
            RemoteBlockSourceConfig::default(),
        ));
        let mut rx = source.subscribe();

        let run = tokio::spawn(source.clone().run());

        // The 12..=19 hole is healed; the broadcast climbs ascending to 20 and
        // every height 10..=20 ends up durable.
        let seen = drain_until(&mut rx, 20).await;
        assert_eq!(*seen.first().unwrap(), 10);
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(20));
        for height in 12..=19 {
            assert!(store.get(height).await.unwrap().is_some());
        }

        run.abort();
    }

    #[tokio::test]
    async fn contiguous_frontier_none_until_stored() {
        let store = Arc::new(MemoryBlockStore::default());
        let source = source_with(store.clone());
        assert_eq!(source.contiguous_frontier().await.unwrap(), None);
        store.put(1, &block(1)).await.unwrap();
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(1));
    }

    #[tokio::test]
    async fn drain_live_reconnects_after_stream_end() {
        let store = Arc::new(MemoryBlockStore::default());
        for height in 1..=9 {
            store.put(height, &block(height)).await.unwrap();
        }

        // Episode 1: tip 10, deliver 10, 11, then the stream ENDS (connection
        // drop). Episode 2: tip 20 (12..=19 produced during the downtime),
        // deliver 20, then pend.
        let subscriber = Arc::new(ScriptedSubscriber::new(vec![
            (vec![10, 11], false),
            (vec![20], true),
        ]));
        let config = RemoteBlockSourceConfig {
            reconnect_backoff: Duration::from_millis(10),
            ..Default::default()
        };
        let source = Arc::new(RemoteBlockSource::new(
            store.clone(),
            subscriber,
            Arc::new(MockFetcher),
            config,
        ));
        let mut rx = source.subscribe();

        let run = tokio::spawn(source.clone().run());

        // 10, 11 from episode 1; the stream drops, the source reconnects, the
        // healer fills the 12..=19 downtime hole, and the broadcast climbs to 20.
        let seen = drain_until(&mut rx, 20).await;
        assert_eq!(*seen.first().unwrap(), 10);
        assert_eq!(source.contiguous_frontier().await.unwrap(), Some(20));
        for height in 12..=19 {
            assert!(store.get(height).await.unwrap().is_some());
        }

        run.abort();
    }
}
