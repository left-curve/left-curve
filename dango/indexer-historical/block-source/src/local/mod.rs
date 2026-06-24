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
/// Runs on the same host as the dango node; obtains blocks from the node's
/// in-process GraphQL subscription and from the cache files the node already
/// writes to disk. See `design/local-block-source.md` for the full design.
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

    /// Catch up the frontier to `height`, loading any missing blocks from
    /// the on-disk cache and broadcasting them in order.
    async fn consume_height(&self, height: u64) -> AnyResult<()> {
        let mut current = self.frontier.load(Ordering::Acquire);

        // Subscription re-emits the last indexed block as its first event;
        // skip anything we already passed.
        if height <= current {
            return Ok(());
        }

        // Normally `height == current + 1`, but a race between the boot-time
        // query and the subscription open (or a temporary disconnect) can
        // leave a gap that we close here one block at a time.
        while current < height {
            let next = current + 1;
            match self.get(next).await? {
                Some(block) => {
                    self.frontier.store(next, Ordering::Release);
                    let _ = self.broadcast_tx.send(Arc::new(block));
                    current = next;
                },
                None => {
                    // The node writes the cache file before publishing on
                    // pubsub, so this should be vanishingly rare. Bail out of
                    // the catch-up loop and let the next subscription event
                    // retry.
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        height = next,
                        "cache file not yet on disk; will retry on next event"
                    );
                    break;
                },
            }
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

        // Outer loop: keep the subscription open across transient
        // disconnects. Any gap accumulated during a disconnect is closed
        // automatically by the catch-up greedy logic in `consume_height`,
        // which fires off the first re-emitted event after reconnect.
        loop {
            let mut stream = match self.httpd_client.subscribe_blocks().await {
                Ok(s) => s,
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        error = %_e,
                        backoff_secs = RECONNECT_BACKOFF.as_secs(),
                        "subscribe_blocks failed, retrying"
                    );
                    tokio::time::sleep(RECONNECT_BACKOFF).await;
                    continue;
                },
            };

            #[cfg(feature = "tracing")]
            tracing::info!("subscription open");

            loop {
                match stream.next().await {
                    Some(Ok(height)) => self.consume_height(height).await?,
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
