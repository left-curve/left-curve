use {
    crate::{error::Result, pubsub::PubSub},
    async_trait::async_trait,
    std::{
        collections::HashSet,
        pin::Pin,
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    },
    tokio::sync::broadcast,
    tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream},
};

/// Simple deduplication cache for block notifications
#[derive(Debug)]
struct DeduplicationCache {
    published_blocks: Arc<Mutex<HashSet<u64>>>,
    last_cleanup: Arc<Mutex<Instant>>,
}

impl DeduplicationCache {
    fn new() -> Self {
        Self {
            published_blocks: Arc::new(Mutex::new(HashSet::new())),
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }

    fn try_publish(&self, block_height: u64) -> bool {
        let mut published_blocks = self.published_blocks.lock().unwrap();

        // Clean up old entries periodically (keep last 1000 blocks)
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        if last_cleanup.elapsed() > Duration::from_secs(60) {
            if published_blocks.len() > 1000 {
                let min_height = block_height.saturating_sub(1000);
                published_blocks.retain(|&h| h >= min_height);
            }
            *last_cleanup = Instant::now();
        }

        // Try to insert, returns false if already exists
        published_blocks.insert(block_height)
    }
}

static MEMORY_DEDUP_CACHE: std::sync::OnceLock<DeduplicationCache> = std::sync::OnceLock::new();

/// In-memory pubsub implementation.
pub struct MemoryPubSub {
    pub sender: broadcast::Sender<u64>,
}

impl MemoryPubSub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);

        Self { sender }
    }
}

#[async_trait]
impl PubSub for MemoryPubSub {
    async fn subscribe_block_minted(&self) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>> {
        let rx = self.sender.subscribe();

        Ok(Box::pin(
            BroadcastStream::new(rx).filter_map(|res| res.ok()),
        ))
    }

    async fn publish_block_minted(&self, block_height: u64) -> Result<usize> {
        let cache = MEMORY_DEDUP_CACHE.get_or_init(|| DeduplicationCache::new());

        // Only publish if this block height hasn't been published recently
        if cache.try_publish(block_height) {
            // NOTE: Discarding the error as it happens if no receivers are connected.
            // There is no way to know if there are any receivers connected without RACE conditions.
            let sent = self.sender.send(block_height).unwrap_or_default();

            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Published block_minted notification for block {}",
                block_height
            );

            Ok(sent)
        } else {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Skipped duplicate block_minted notification for block {}",
                block_height
            );

            Ok(0)
        }
    }
}
