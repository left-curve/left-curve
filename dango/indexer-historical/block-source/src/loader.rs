//! [`BlockLoader`] — an async-graphql [`DataLoader`] over [`BlockSource::get`].
//!
//! GraphQL read surfaces hydrate a unit's raw payload (the full `Tx`, its
//! `TxOutcome`, non-priority event data) from the block it lives in. Done
//! naively that is an N+1: a page of N rows each fetches — and borsh-decodes —
//! its (often shared) block, twice over if it asks for both the tx and its
//! outcome. Keying the load on `block_height` collapses that to one decode per
//! distinct block per request: resolvers call [`DataLoader::load_one`], the
//! loader batches the heights of one resolution tick into a single [`load`],
//! and the whole block — shared behind an [`Arc`] — is handed to every row that
//! lives in it. Blocks are immutable, so the loader's cache is always coherent.
//!
//! [`DataLoader`]: async_graphql::dataloader::DataLoader
//! [`DataLoader::load_one`]: async_graphql::dataloader::DataLoader::load_one
//! [`load`]: Loader::load

use {
    crate::BlockSource,
    async_graphql::dataloader::Loader,
    dango_indexer_historical_types::BlockData,
    std::{collections::HashMap, sync::Arc},
};

/// A [`DataLoader`](async_graphql::dataloader::DataLoader) source reading whole
/// blocks by height from a [`BlockSource`]. Register one per schema —
/// `DataLoader::new(BlockLoader::new(source), tokio::spawn)` — then resolvers
/// pull `DataLoader<BlockLoader>` from the context and `load_one(height)`.
pub struct BlockLoader {
    source: Arc<dyn BlockSource>,
}

impl BlockLoader {
    #[must_use]
    pub fn new(source: Arc<dyn BlockSource>) -> Self {
        Self { source }
    }
}

impl Loader<u64> for BlockLoader {
    type Error = Arc<anyhow::Error>;
    /// The whole block, shared so every row in it clones an `Arc`, not the
    /// payload.
    type Value = Arc<BlockData>;

    async fn load(&self, heights: &[u64]) -> Result<HashMap<u64, Self::Value>, Self::Error> {
        // The DataLoader hands us distinct keys; fetch them concurrently. A
        // height the source can't supply is simply absent from the map, so
        // `load_one` yields `None` rather than an error.
        let loads = heights.iter().map(|&height| {
            let source = Arc::clone(&self.source);
            async move {
                anyhow::Ok(
                    source
                        .get(height)
                        .await?
                        .map(|block| (height, Arc::new(block))),
                )
            }
        });
        let blocks = futures::future::try_join_all(loads)
            .await
            .map_err(Arc::new)?;
        Ok(blocks.into_iter().flatten().collect())
    }
}
