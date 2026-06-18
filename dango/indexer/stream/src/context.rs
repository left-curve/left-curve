use crate::{perps_events::PerpsEventBlock, recent_stream::RecentStream};

/// A cheap-to-clone reader handle to the realtime stream's in-memory state,
/// held by the httpd server and registered as GraphQL schema data. The
/// `perps_events2` subscription resolver uses it to open subscriptions.
///
/// Cloning shares the underlying ring + broadcast with the live [`Indexer`].
///
/// [`Indexer`]: crate::Indexer
#[derive(Clone)]
pub struct Context {
    perps: RecentStream<PerpsEventBlock>,
}

impl Context {
    pub(crate) fn new(perps: RecentStream<PerpsEventBlock>) -> Self {
        Self { perps }
    }

    /// The perps-events stream backing the `perps_events2` subscription.
    pub fn perps(&self) -> &RecentStream<PerpsEventBlock> {
        &self.perps
    }
}
