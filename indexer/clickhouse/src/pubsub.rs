pub mod memory;

pub use memory::MemoryPubSub;
use {crate::error::Result, async_trait::async_trait, std::pin::Pin, tokio_stream::Stream};

#[async_trait]
pub trait PubSub {
    async fn subscribe_candles_cached(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = u64> + Send + '_>>>;

    async fn publish_candles_cached(&self, block_height: u64) -> Result<usize>;
}

pub enum PubSubType {
    Memory,
}
