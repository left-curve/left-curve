use {
    crate::httpd_client::HttpdClient,
    async_trait::async_trait,
    dango_indexer_historical_types::{AnyResult, BlockData},
    futures::stream::BoxStream,
    reqwest::IntoUrl,
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
    /// Open the subscription and return a stream of fully-assembled blocks,
    /// ascending and contiguous. `since` is the **resume point**: the feed
    /// replays from that height before streaming the live tail — a reconnect
    /// passes `frontier + 1`, so the downtime hole is refilled with no gap —
    /// while `None` starts at the current tip (a fresh, empty store).
    ///
    /// The payload rides the stream itself (the node's `full_block`
    /// subscription), so a detached host needs no local disk. Whatever sits
    /// below the first delivered height is just a gap the healer fills — see
    /// `design/remote-block-source.md`.
    async fn subscribe(
        &self,
        since: Option<u64>,
    ) -> AnyResult<BoxStream<'static, AnyResult<BlockData>>>;
}

// ---- the full_block impl ----

/// Concrete [`LiveSubscriber`] over a node's `full_block` GraphQL subscription.
///
/// The same impl serves **both** block sources — only the base URL differs (the
/// local in-process `dango-httpd` vs a remote sentinel) — a thin wrapper over
/// the shared [`HttpdClient::subscribe_full_blocks`].
///
/// [`HttpdClient::subscribe_full_blocks`]: crate::httpd_client::HttpdClient
pub struct FullBlockSubscriber {
    httpd: HttpdClient,
}

impl FullBlockSubscriber {
    /// Build from the node's base URL (e.g. `http://sentinel:8080`).
    pub fn new<U>(base_url: U) -> AnyResult<Self>
    where
        U: IntoUrl,
    {
        Ok(Self {
            httpd: HttpdClient::new(base_url)?,
        })
    }
}

#[async_trait]
impl LiveSubscriber for FullBlockSubscriber {
    async fn subscribe(
        &self,
        since: Option<u64>,
    ) -> AnyResult<BoxStream<'static, AnyResult<BlockData>>> {
        self.httpd.subscribe_full_blocks(since).await
    }
}
