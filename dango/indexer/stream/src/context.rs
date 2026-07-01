use {
    crate::{perps_events::PerpsEventBlock, recent_stream::RecentStream},
    dango_primitives::FullBlock,
};

/// A cheap-to-clone reader handle to the realtime stream's in-memory state,
/// held by the httpd server. The `/ws` WebSocket handler uses it to open the
/// `perpsEvents` and `fullBlock` channel subscriptions.
///
/// Cloning shares the underlying rings + broadcasts with the live [`Indexer`].
///
/// [`Indexer`]: crate::Indexer
#[derive(Clone)]
pub struct Context {
    perps: RecentStream<PerpsEventBlock>,
    blocks: RecentStream<FullBlock>,
}

impl Context {
    pub(crate) fn new(
        perps: RecentStream<PerpsEventBlock>,
        blocks: RecentStream<FullBlock>,
    ) -> Self {
        Self { perps, blocks }
    }

    /// The perps-events stream backing the `/ws` `perpsEvents` channel.
    pub fn perps(&self) -> &RecentStream<PerpsEventBlock> {
        &self.perps
    }

    /// The full-block stream backing the `/ws` `fullBlock` channel.
    pub fn blocks(&self) -> &RecentStream<FullBlock> {
        &self.blocks
    }
}
