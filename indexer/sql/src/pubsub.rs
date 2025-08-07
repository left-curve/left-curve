use {crate::error::Result, async_trait::async_trait, std::pin::Pin, tokio_stream::Stream};

pub mod memory;
pub mod postgres;
pub mod postgres_multiple;

pub use {memory::MemoryPubSub, postgres::PostgresPubSub};

#[async_trait]
pub trait PubSub<I> {
    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = I> + Send + '_>>>;

    async fn publish(&self, item: I) -> Result<usize>;
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

    #[ignore = "this test requires a running postgres instance and is not meant to be run in CI yet"]
    #[tokio::test]
    async fn test_postgres_pubsub() -> anyhow::Result<()> {
        let db_host = std::env::var("DB_HOST").unwrap_or("localhost".to_string());
        let pool =
            sqlx::PgPool::connect(format!("postgres://postgres@{db_host}/grug_test").as_str())
                .await?;

        let pubsub = PostgresPubSub::new(pool.clone(), "blocks").await?;
        let pubsub_clone = pubsub.clone();

        {
            tokio::task::spawn(async move {
                for idx in 1..10 {
                    pubsub.publish(idx).await?;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }

                Ok::<_, anyhow::Error>(())
            });
        }

        {
            let mut stream = pubsub_clone.subscribe().await?;

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
    #[ignore = "this test requires a running postgres instance and is not meant to be run in CI yet"]
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
