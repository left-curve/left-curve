use {
    crate::{error::Result, pubsub::PubSub},
    async_trait::async_trait,
    sea_orm::sqlx::{self, postgres::PgListener},
    std::pin::Pin,
    tokio::sync::broadcast,
    tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream},
};

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
        sqlx::query("select pg_notify('blocks', json_build_object('block_height', $1)::text)")
            .bind(block_height as i64)
            .execute(&self.pool)
            .await?;

        Ok(1)
    }
}
