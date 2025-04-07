use {
    async_trait::async_trait,
    grug::{Binary, Lengthy, NonEmpty},
    std::{fmt::Display, pin::Pin},
};

#[async_trait]
pub trait PythClientTrait: Clone {
    type Error: Display;

    async fn stream<I>(
        &mut self,
        ids: NonEmpty<I>,
    ) -> Result<Pin<Box<dyn tokio_stream::Stream<Item = Vec<Binary>> + Send>>, Self::Error>
    where
        I: IntoIterator + Lengthy + Send + Clone,
        I::Item: ToString;

    fn get_latest_vaas<I>(&self, ids: NonEmpty<I>) -> Result<Vec<Binary>, Self::Error>
    where
        I: IntoIterator + Clone + Lengthy,
        I::Item: ToString;

    fn close(&mut self);
}
