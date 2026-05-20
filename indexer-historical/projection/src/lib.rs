//! [`Projection`] trait for the historical indexer.
//!
//! A projection is a self-contained consumer of the block stream. Each
//! projection owns its own tables, its own watermark, and (optionally) its
//! own backend. The app's `projection_loop` drives one projection at a
//! time, alternating between pull catch-up via `BlockSource::get` and push
//! live via the broadcast channel.
//!
//! See `DESIGN.md` for the surrounding architecture.

use {
    async_trait::async_trait,
    indexer_historical_types::{AnyResult, BlockData},
};

/// Domain-specific consumer of the block stream.
#[async_trait]
pub trait Projection: Send + Sync {
    /// Stable id used to persist this projection's watermark.
    ///
    /// Bumping the id forces a full re-backfill: a new id starts with an
    /// empty watermark, and the projection catches up from `min_height()`.
    fn id(&self) -> &'static str;

    /// Minimum height below which this projection has nothing to do
    /// (e.g. a contract that didn't exist before that block).
    fn min_height(&self) -> u64 {
        0
    }

    /// Process one block.
    ///
    /// Implementations write to their own tables and update their own
    /// watermark. Where the backend supports it (Postgres), both should
    /// happen in the same transaction. ClickHouse-backed projections need
    /// a different atomicity strategy (see `DESIGN.md` open questions).
    async fn process(&self, block: &BlockData) -> AnyResult<()>;

    /// Last height fully processed by this projection.
    ///
    /// `None` if the projection has not run yet (or its watermark id was
    /// just bumped).
    async fn last_processed_height(&self) -> AnyResult<Option<u64>>;
}
