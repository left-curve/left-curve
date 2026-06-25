use {
    crate::{BlockSource, httpd_client::HttpdClient},
    async_trait::async_trait,
    dango_indexer_cache::{IndexerPath, cache_file::CacheFile},
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::StreamExt,
    reqwest::IntoUrl,
    std::{
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    },
    tokio::sync::broadcast,
};

/// Backoff between subscription / init reconnection attempts.
const RECONNECT_BACKOFF: Duration = Duration::from_secs(5);

/// V1 [`BlockSource`] implementation.
///
/// Runs on the same host as the dango node. The **live tail** comes from the
/// node's in-process `full_block` subscription (the same shared path the remote
/// source uses); **historical** blocks come from the cache files the node
/// already writes to disk, served by `get`. See `design/local-block-source.md`.
pub struct LocalBlockSource {
    cache_path: IndexerPath,
    httpd_client: HttpdClient,
    /// Highest contiguous block height observed.
    ///
    /// Mutated only by the `run()` task; reads are lock-free. The atomic is
    /// here for cross-task visibility, not for write serialisation.
    frontier: AtomicU64,
    broadcast_tx: broadcast::Sender<Arc<BlockData>>,
}

impl LocalBlockSource {
    pub fn new<U>(
        cache_path: IndexerPath,
        httpd_base_url: U,
        pubsub_buffer_size: usize,
    ) -> AnyResult<Self>
    where
        U: IntoUrl,
    {
        let httpd_client = HttpdClient::new(httpd_base_url)?;
        let (broadcast_tx, _) = broadcast::channel(pubsub_buffer_size);
        Ok(Self {
            cache_path,
            httpd_client,
            frontier: AtomicU64::new(0),
            broadcast_tx,
        })
    }

    /// Query `dango-httpd` for the latest indexed block height and seed the
    /// frontier accordingly. Called once from `run()` at startup.
    async fn init_frontier(&self) -> AnyResult<()> {
        if let Some(h) = self.httpd_client.latest_block_height().await? {
            self.frontier.store(h, Ordering::Release);
        }
        Ok(())
    }
}

#[async_trait]
impl BlockSource for LocalBlockSource {
    async fn run(self: Arc<Self>) -> AnyResult<()> {
        #[cfg(feature = "tracing")]
        tracing::info!("LocalBlockSource starting");

        // Init frontier — retry on transient failure (e.g. dango-httpd not
        // yet up when we start).
        loop {
            match self.init_frontier().await {
                Ok(()) => break,
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        error = %_e,
                        backoff_secs = RECONNECT_BACKOFF.as_secs(),
                        "init_frontier failed, retrying"
                    );
                    tokio::time::sleep(RECONNECT_BACKOFF).await;
                },
            }
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            frontier = self.frontier.load(Ordering::Acquire),
            "frontier initialised"
        );

        // Outer loop: keep the `full_block` subscription open across transient
        // disconnects. Each (re)connect resumes from `frontier + 1`, so the node
        // replays any downtime hole before streaming the live tail — the same
        // shared path the remote source uses.
        loop {
            let since = match self.frontier.load(Ordering::Acquire) {
                0 => None,
                frontier => Some(frontier + 1),
            };

            let mut stream = match self.httpd_client.subscribe_full_blocks(since).await {
                Ok(s) => s,
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        error = %_e,
                        backoff_secs = RECONNECT_BACKOFF.as_secs(),
                        "subscribe_full_blocks failed, retrying"
                    );
                    tokio::time::sleep(RECONNECT_BACKOFF).await;
                    continue;
                },
            };

            #[cfg(feature = "tracing")]
            tracing::info!("subscription open");

            loop {
                match stream.next().await {
                    Some(Ok(block)) => {
                        let height = block.height();
                        let current = self.frontier.load(Ordering::Acquire);

                        // The feed delivers in order from `since`. Skip a
                        // re-delivered block at/under the frontier; on a gap (a
                        // dropped block) reconnect to replay from `frontier + 1`
                        // rather than broadcasting a hole.
                        if height <= current {
                            continue;
                        }
                        if height > current + 1 {
                            #[cfg(feature = "tracing")]
                            tracing::warn!(current, height, "gap in full_block feed; reconnecting");
                            break;
                        }

                        self.frontier.store(height, Ordering::Release);
                        let _ = self.broadcast_tx.send(Arc::new(block));
                    },
                    Some(Err(_e)) => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(error = %_e, "subscription error, reconnecting");
                        break;
                    },
                    None => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!("subscription stream ended, reconnecting");
                        break;
                    },
                }
            }

            tokio::time::sleep(RECONNECT_BACKOFF).await;
        }
    }

    async fn get(&self, height: u64) -> AnyResult<Option<BlockData>> {
        let path = self.cache_path.block_path(height);

        if !CacheFile::exists(path.clone()) {
            return Ok(None);
        }

        let cache_file = CacheFile::load_from_disk_async(path).await?;

        Ok(Some(BlockData {
            block: cache_file.data.block,
            outcome: cache_file.data.block_outcome,
        }))
    }

    fn subscribe(&self) -> broadcast::Receiver<Arc<BlockData>> {
        self.broadcast_tx.subscribe()
    }

    async fn contiguous_frontier(&self) -> AnyResult<Option<u64>> {
        let h = self.frontier.load(Ordering::Acquire);
        Ok(if h == 0 {
            None
        } else {
            Some(h)
        })
    }
}
