use {
    async_trait::async_trait,
    grug::{Lengthy, NonEmpty},
    pyth_types::PriceUpdate,
    std::{fmt::Display, pin::Pin},
};

#[async_trait]
pub trait PythClientTrait: Clone {
    type Error: Display;

    /// Creates a stream of price updates for the given ids.
    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = PriceUpdate> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString;

    /// Retrieves the latest price update for the given ids.
    fn get_latest_price_update<I>(&self, ids: NonEmpty<I>) -> Result<PriceUpdate, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString;

    /// Closes the stream.
    fn close(&mut self);
}
