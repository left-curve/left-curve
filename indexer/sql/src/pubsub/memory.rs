use {
    crate::{error::Result, pubsub::PubSub},
    async_trait::async_trait,
    std::pin::Pin,
    tokio::sync::broadcast,
    tokio_stream::{Stream, StreamExt, wrappers::BroadcastStream},
};

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
        // NOTE: Discarding the error as it happens if no receivers are connected.
        // There is no way to know if there are any receivers connected without RACE conditions.
        Ok(self.sender.send(block_height).unwrap_or_default())
    }
}
