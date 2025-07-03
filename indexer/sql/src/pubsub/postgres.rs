use {
    crate::{error::Result, pubsub::PubSub},
    async_trait::async_trait,
    sea_orm::sqlx::{self, postgres::PgListener},
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

static POSTGRES_DEDUP_CACHE: std::sync::OnceLock<DeduplicationCache> = std::sync::OnceLock::new();

#[derive(Clone)]
pub struct PostgresPubSub {
    sender: broadcast::Sender<u64>,
    pool: sqlx::PgPool,
}

impl PostgresPubSub {
    pub async fn new(pool: sqlx::PgPool) -> Result<Self> {
        let (sender, _) = broadcast::channel::<u64>(128);

        let result = Self { sender, pool };

        result.connect().await?;

        Ok(result)
    }

    async fn connect(&self) -> Result<()> {
        let sender = self.sender.clone();
        let mut listener = PgListener::connect_with(&self.pool).await?;

        tokio::spawn(async move {
            loop {
                if let Err(_e) = listener.listen("blocks").await {
                    #[cfg(feature = "tracing")]
                    tracing::error!(error = %_e, "Listen error");

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    continue;
                }

                #[cfg(feature = "tracing")]
                tracing::info!("Connected to PostgreSQL notifications");

                #[allow(clippy::while_let_loop)]
                loop {
                    match listener.recv().await {
                        Ok(notification) => {
                            if let Ok(data) =
                                serde_json::from_str::<serde_json::Value>(notification.payload())
                            {
                                if let Some(block_height) =
                                    data.get("block_height").and_then(|v| v.as_u64())
                                {
                                    let _ = sender.send(block_height); // Ignore send errors
                                }
                            }
                        },
                        Err(_e) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!(error = %_e, "Notification error");

                            break;
                        },
                    }
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl PubSub for PostgresPubSub {
    async fn subscribe_block_minted(&self) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>> {
        let rx = self.sender.subscribe();

        Ok(Box::pin(
            BroadcastStream::new(rx).filter_map(|res| res.ok()),
        ))
    }

    async fn publish_block_minted(&self, block_height: u64) -> Result<usize> {
        let cache = POSTGRES_DEDUP_CACHE.get_or_init(|| DeduplicationCache::new());

        // Only publish if this block height hasn't been published recently
        if cache.try_publish(block_height) {
            sqlx::query("select pg_notify('blocks', json_build_object('block_height', $1)::text)")
                .bind(block_height as i64)
                .execute(&self.pool)
                .await?;

            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Published block_minted notification for block {}",
                block_height
            );

            Ok(1)
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
