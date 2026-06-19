use {
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::stream::BoxStream,
};

/// Follows a sentinel's chain tip and yields the live block stream.
///
/// Always a sentinel subscription — the tip only ever comes from a node, never
/// from the cold archive — so there is one logical implementation; behind a
/// trait so the [`RemoteBlockSource`] coordinator can be tested with a mock.
///
/// [`RemoteBlockSource`]: crate::RemoteBlockSource
#[async_trait]
pub trait LiveSubscriber: Send + Sync {
    /// Open the subscription. Returns the current tip height `L` together with
    /// a stream of fully-assembled blocks whose **first item is block `L`**,
    /// continuing ascending and contiguous (the chain produces — and the
    /// sentinel delivers — in order).
    ///
    /// The payload is fetched by the subscriber: the notification carries only
    /// the height, and a detached host has no local disk to read it from. The
    /// `L`-starts-the-stream contract is what lets the source treat `[1, L)` as
    /// the fetcher's job and `[L, ∞)` as the subscriber's, with no gap at the
    /// seam — see `design/remote-block-source.md`.
    async fn subscribe(&self) -> AnyResult<(u64, BoxStream<'static, AnyResult<BlockData>>)>;
}
