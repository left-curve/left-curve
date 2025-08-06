use grug::{QueryClient, Signer};

use crate::auth::Nonce;

#[async_trait::async_trait]
/// Represent a signer that can query and update its nonce.
pub trait SequencedSigner: Signer {
    async fn query_nonce<C>(&self, client: &C) -> anyhow::Result<Nonce>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>;

    async fn update_nonce<C>(&mut self, client: &C) -> anyhow::Result<()>
    where
        C: QueryClient,
        anyhow::Error: From<C::Error>;
}
