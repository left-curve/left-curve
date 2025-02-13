use {
    crate::error::IndexerError,
    async_stream::stream,
    async_trait::async_trait,
    sea_orm::sqlx::{self, postgres::PgListener},
    std::pin::Pin,
    tokio::sync::broadcast,
    tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt},
};

/// In-memory pubsub implementation
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
    async fn subscribe_block_minted(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>, IndexerError> {
        let rx = self.sender.subscribe();
        Ok(Box::pin(
            BroadcastStream::new(rx).filter_map(|res| res.ok()),
        ))
    }

    async fn publish_block_minted(&self, block_height: u64) -> Result<usize, IndexerError> {
        // NOTE: discarding the error as it happens if no receivers are connected
        // There is no way to know if there are any receivers connected without RACE conditions
        Ok(self.sender.send(block_height).unwrap_or_default())
    }
}

#[derive(Clone)]
pub struct PostgresPubSub {
    pool: sqlx::PgPool,
}

impl PostgresPubSub {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PubSub for PostgresPubSub {
    async fn subscribe_block_minted(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>, IndexerError> {
        let mut listener = PgListener::connect_with(&self.pool).await?;

        listener.listen("blocks").await?;

        let stream = stream! {
            loop {
                match listener.recv().await {
                    Ok(notification) => {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(notification.payload()) {
                            if let Some(block_height) = data.get("block_height") {
                                if let Some(block_height) = block_height.as_u64() {
                                    yield block_height;
                                }
                            }
                        }
                    },
                    Err(e) => {
                        #[cfg(feature = "tracing")]
                        tracing::error!("Error receiving notification: {e:?}");

                        break;
                    },
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn publish_block_minted(&self, block_height: u64) -> Result<usize, IndexerError> {
        sqlx::query("select pg_notify('blocks', json_build_object('block_height', $1)::text)")
            .bind(block_height as i64)
            .execute(&self.pool)
            .await?;

        Ok(1)
    }
}

#[async_trait]
pub trait PubSub {
    async fn subscribe_block_minted(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>, IndexerError>;

    async fn publish_block_minted(&self, block_height: u64) -> Result<usize, IndexerError>;
}

pub enum PubSubType {
    Memory,
    Postgres,
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*, assertor::*, sqlx::postgres::PgPoolOptions, std::time::Duration,
        tokio_stream::StreamExt,
    };

    #[ignore]
    #[tokio::test]
    async fn test_postgres_pubsub() -> anyhow::Result<()> {
        let pool = sqlx::PgPool::connect("postgres://postgres@postgres/grug_test").await?;

        let pubsub = PostgresPubSub::new(pool.clone());
        let pubsub_clone = pubsub.clone();

        {
            tokio::task::spawn(async move {
                for idx in 1..10 {
                    pubsub.publish_block_minted(idx).await?;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }

                Ok::<_, anyhow::Error>(())
            });
        }

        {
            let mut stream = pubsub_clone.subscribe_block_minted().await?;

            tokio::select! {
                block_height = stream.next() => {
                    assert_that!(block_height.unwrap_or_default()).is_equal_to(1);
                }
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    println!("timeout waiting");
                }
            }
        }

        Ok(())
    }

    /// A test that demonstrates how to use postgres pubsub.
    #[ignore]
    #[tokio::test]
    async fn manual_psql_implementation() -> anyhow::Result<()> {
        let pool = PgPoolOptions::new()
            .connect("postgres://postgres@postgres/grug_test")
            .await?;

        // Start listening on a channel
        let mut listener = PgListener::connect_with(&pool).await?;
        listener.listen("blocks").await?;

        tokio::task::spawn(async move {
            for idx in 1..10 {
                sqlx::query(
                    "select pg_notify('blocks', json_build_object('block_height', $1)::text)",
                )
                .bind(idx)
                .execute(&pool)
                .await?;
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            anyhow::Ok(())
        });

        for _ in 1..=3 {
            tokio::select! {
                notification = listener.recv() => {
                    println!("Got notification: {notification:?}");
                }
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    println!("Timeout waiting");
                }
            }
        }

        Ok(())
    }
}
