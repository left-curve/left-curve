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
        // The DataLoader hands us distinct keys; fetch them concurrently. Each
        // height is isolated: one the source can't supply (`Ok(None)`) or that
        // *errors* is simply absent from the map, so `load_one` yields `None`
        // rather than sinking the whole batch — one unreadable block can't fail
        // the sibling rows of a page (the affected row sees the same "not
        // available" as a miss). A hard error is logged so it stays visible.
        let loads = heights.iter().map(|&height| {
            let source = Arc::clone(&self.source);
            async move { (height, source.get(height).await) }
        });
        let results = futures::future::join_all(loads).await;

        let mut blocks = HashMap::with_capacity(results.len());
        for (height, result) in results {
            match result {
                Ok(Some(block)) => {
                    blocks.insert(height, Arc::new(block));
                },
                Ok(None) => { /* missing — absent from the map, resolves to None */ },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        height,
                        error = %_err,
                        "block load failed; treating as unavailable",
                    );
                },
            }
        }
        Ok(blocks)
    }
}
