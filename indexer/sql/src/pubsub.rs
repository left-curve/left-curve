use {crate::error::IndexerError, async_trait::async_trait, std::pin::Pin, tokio_stream::Stream};

pub mod memory;
pub mod postgres;
pub mod postgres_multiple;

pub use {memory::MemoryPubSub, postgres::PostgresPubSub};

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
        super::*,
        assertor::*,
        sqlx::postgres::{PgListener, PgPoolOptions},
        std::time::Duration,
        tokio_stream::StreamExt,
    };

    #[ignore]
    #[tokio::test]
    async fn test_postgres_pubsub() -> anyhow::Result<()> {
        let db_host = std::env::var("DB_HOST").unwrap_or("localhost".to_string());
        let pool =
            sqlx::PgPool::connect(format!("postgres://postgres@{db_host}/grug_test").as_str())
                .await?;

        let pubsub = PostgresPubSub::new(pool.clone()).await?;
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
    async fn foobar() -> anyhow::Result<()> {
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
