use {
    crate::{error::Result, pubsub::PubSub},
    async_trait::async_trait,
    sea_orm::sqlx::{self, postgres::PgListener},
    serde::{Serialize, de::DeserializeOwned},
    std::pin::Pin,
    tokio::sync::broadcast,
    tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream},
};

#[derive(Clone)]
pub struct PostgresPubSub<I>
where
    I: Clone + Send + 'static,
{
    sender: broadcast::Sender<I>,
    pool: sqlx::PgPool,
    name: &'static str,
}

impl<I> PostgresPubSub<I>
where
    I: Clone + Send + 'static,
    I: serde::de::DeserializeOwned,
{
    pub async fn new(pool: sqlx::PgPool, name: &'static str) -> Result<Self> {
        let (sender, _) = broadcast::channel::<I>(128);

        let result = Self { sender, pool, name };

        result.connect().await?;

        Ok(result)
    }

    async fn connect(&self) -> Result<()> {
        let sender = self.sender.clone();
        let mut listener = PgListener::connect_with(&self.pool).await?;

        let name = self.name.to_string();

        tokio::spawn(async move {
            loop {
                if let Err(_e) = listener.listen(&name).await {
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
                            if let Ok(item) = serde_json::from_str::<I>(notification.payload()) {
                                let _ = sender.send(item); // Ignore send errors
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
impl<I> PubSub<I> for PostgresPubSub<I>
where
    I: Clone + Send + Sync + Serialize + DeserializeOwned,
{
    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = I> + Send + '_>>> {
        let rx = self.sender.subscribe();

        Ok(Box::pin(
            BroadcastStream::new(rx).filter_map(|res| res.ok()),
        ))
    }

    async fn publish(&self, item: I) -> Result<usize> {
        let json_data = serde_json::to_string(&item)?;

        sqlx::query("select pg_notify($1, $2)")
            .bind(self.name)
            .bind(json_data)
            .execute(&self.pool)
            .await?;

        Ok(1)
    }
}
