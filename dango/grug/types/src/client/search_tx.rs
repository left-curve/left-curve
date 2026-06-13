use {
    crate::{Hash256, SearchTxOutcome},
    async_trait::async_trait,
};

#[async_trait]
pub trait SearchTxClient {
    type Error;

    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error>;
}
