use {
    crate::error::Error,
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

impl PubSub for MemoryPubSub {
    fn subscribe_block_minted(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
        let rx = self.sender.subscribe();
        Box::pin(BroadcastStream::new(rx).filter_map(|res| res.ok()))
    }

    fn publish_block_minted(&self, block_height: u64) -> Result<usize, Error> {
        // NOTE: discarding the error as it happens if no receivers are connected
        // There is no way to know if there are any receivers connected without RACE conditions
        Ok(self.sender.send(block_height).unwrap_or_default())
    }
}

pub trait PubSub {
    fn subscribe_block_minted(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;
    fn publish_block_minted(&self, block_height: u64) -> Result<usize, Error>;
}
