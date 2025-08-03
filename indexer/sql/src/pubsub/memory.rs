use {
    crate::{error::Result, pubsub::PubSub},
    async_trait::async_trait,
    std::pin::Pin,
    tokio::sync::broadcast,
    tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream},
};

/// In-memory pubsub implementation.
pub struct MemoryPubSub<I> {
    pub sender: broadcast::Sender<I>,
}

impl<I> MemoryPubSub<I>
where
    I: Clone + Send + 'static,
{
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);

        Self { sender }
    }
}

#[async_trait]
impl<I> PubSub<I> for MemoryPubSub<I>
where
    I: Clone + Send + 'static,
{
    async fn subscribe(&self) -> Result<Pin<Box<dyn Stream<Item = I> + Send + '_>>> {
        let rx = self.sender.subscribe();

        Ok(Box::pin(
            BroadcastStream::new(rx).filter_map(|res| res.ok()),
        ))
    }

    async fn publish(&self, item: I) -> Result<usize> {
        // NOTE: Discarding the error as it happens if no receivers are connected.
        // There is no way to know if there are any receivers connected without RACE conditions.
        Ok(self.sender.send(item).unwrap_or_default())
    }
}
