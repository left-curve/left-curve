//! The per-block payload for the `full_block` subscription: a finalized block
//! (header + transactions) paired with its execution outcome (all events).
//!
//! This is the second [`RecentStream`] item type the crate docs anticipated —
//! `RecentStream<BlockAndOutcome>` is the in-memory ring behind the "new blocks"
//! subscription, the block-level sibling of `RecentStream<PerpsEventBlock>`.
//!
//! [`RecentStream`]: crate::RecentStream

use {
    crate::recent_stream::HasHeight,
    dango_primitives::{Block, BlockOutcome},
    serde::{Deserialize, Serialize},
};

/// A finalized block together with its execution outcome, broadcast as one item
/// per height to `full_block` subscribers.
///
/// Carries the full `Block` (`BlockInfo` + `txs`) and the full `BlockOutcome`
/// (every tx/cron outcome, hence every event). Deliberately does NOT carry the
/// `http_request_details` that `BlockAndBlockOutcomeWithHttpDetails` holds —
/// those contain client IPs and are never exposed over the public API (the REST
/// `/block/info` and `/block/result` routes omit them too).
///
/// Exposed to clients as an `async_graphql::Json` scalar, so it only needs serde
/// derives — there is no async-graphql object mirror of the deep
/// `Tx`/`Message`/`Event`/`Outcome` tree to maintain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAndOutcome {
    pub block: Block,
    pub block_outcome: BlockOutcome,
}

impl HasHeight for BlockAndOutcome {
    fn height(&self) -> u64 {
        self.block.info.height
    }
}
