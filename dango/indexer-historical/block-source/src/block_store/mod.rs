use {
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
};

mod memory;

pub use memory::MemoryBlockStore;

/// The source's durable store of raw blocks, plus the topology queries the
/// coordinator needs (resume point + gaps to backfill).
///
/// Concrete in production (one Postgres `blocks_raw` table); behind a trait so
/// the [`RemoteBlockSource`] coordinator can be built and tested against the
/// in-memory [`MemoryBlockStore`] without a database.
///
/// [`RemoteBlockSource`]: crate::RemoteBlockSource
#[async_trait]
pub trait BlockStore: Send + Sync {
    /// Persist one block. Idempotent: re-putting a height is a no-op, so the
    /// two writers (fetcher backfill + live subscriber) may overlap at a
    /// boundary without harm.
    async fn put(&self, height: u64, data: &BlockData) -> AnyResult<()>;

    /// Read one block by height, or `None` if not stored yet.
    async fn get(&self, height: u64) -> AnyResult<Option<BlockData>>;

    /// Highest `H` such that every height in `[floor, H]` is present, or `None`
    /// if `floor` itself is missing. Seeds the frontier at boot.
    async fn max_contiguous(&self, floor: u64) -> AnyResult<Option<u64>>;

    /// Maximal missing height ranges (inclusive bounds) within `[from, to)`,
    /// ascending. Drives the per-gap backfill.
    async fn gaps(&self, from: u64, to: u64) -> AnyResult<Vec<(u64, u64)>>;
}
