use {
    std::pin::Pin,
    tokio::sync::broadcast,
    tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt},
};

/// In-memory pubsub implementation
#[derive(Debug, Clone)]
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
    fn block_minted(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
        let rx = self.sender.subscribe();
        Box::pin(BroadcastStream::new(rx).filter_map(|res| res.ok()))
    }

    fn publish_new_block(&self, block_height: u64) {
        let _ = self.sender.send(block_height);
    }
}

pub trait PubSub: std::fmt::Debug + Send + Sync {
    fn block_minted(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;
    fn publish_new_block(&self, block_height: u64);
}
