use crate::{
    block_and_outcome::BlockAndOutcome, perps_events::PerpsEventBlock, recent_stream::RecentStream,
};

/// A cheap-to-clone reader handle to the realtime stream's in-memory state,
/// held by the httpd server and registered as GraphQL schema data. The
/// `perps_events2` and `full_block` subscription resolvers use it to open
/// subscriptions.
///
/// Cloning shares the underlying rings + broadcasts with the live [`Indexer`].
///
/// [`Indexer`]: crate::Indexer
#[derive(Clone)]
pub struct Context {
    perps: RecentStream<PerpsEventBlock>,
    blocks: RecentStream<BlockAndOutcome>,
}

impl Context {
    pub(crate) fn new(
        perps: RecentStream<PerpsEventBlock>,
        blocks: RecentStream<BlockAndOutcome>,
    ) -> Self {
        Self { perps, blocks }
    }

    /// The perps-events stream backing the `perps_events2` subscription.
    pub fn perps(&self) -> &RecentStream<PerpsEventBlock> {
        &self.perps
    }

    /// The full-block stream backing the `full_block` subscription.
    pub fn blocks(&self) -> &RecentStream<BlockAndOutcome> {
        &self.blocks
    }
}
