use {
    crate::{error::Result, pubsub::PubSub},
    async_stream::stream,
    async_trait::async_trait,
    sea_orm::sqlx::{self, postgres::PgListener},
    std::pin::Pin,
    tokio_stream::Stream,
};

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
impl PubSub<u64> for PostgresPubSub {
    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>> {
        let mut listener = PgListener::connect_with(&self.pool).await?;

        listener.listen("blocks").await?;

        let stream = stream! {
            #[allow(clippy::while_let_loop)]
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
                    Err(_e) => {
                        #[cfg(feature = "tracing")]
                        tracing::error!(error = %_e, "Error receiving notification");

                        break;
                    },
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn publish(&self, block_height: u64) -> Result<usize> {
        sqlx::query("select pg_notify('blocks', json_build_object('block_height', $1)::text)")
            .bind(block_height as i64)
            .execute(&self.pool)
            .await?;

        Ok(1)
    }
}
