use {
    dango_indexer_historical_types::BlockData,
    dango_primitives::{Block, BlockOutcome},
    serde::Deserialize,
};

/// The on-the-wire `{ block, block_outcome }` shape the sentinel serves — its
/// `BlockAndOutcome`, returned both by the `/block/full/*` REST routes and the
/// `full_block` subscription's JSON scalar. Decoded into [`BlockData`] here, so
/// the historical crates do not depend on the node's `dango-indexer-stream`
/// crate just for this type.
#[derive(Deserialize)]
pub(crate) struct FullBlock {
    pub block: Block,
    pub block_outcome: BlockOutcome,
}

impl From<FullBlock> for BlockData {
    fn from(full: FullBlock) -> Self {
        Self {
            block: full.block,
            outcome: full.block_outcome,
        }
    }
}
