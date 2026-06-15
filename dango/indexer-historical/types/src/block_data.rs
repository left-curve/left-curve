use dango_primitives::{Block, BlockOutcome};

/// Carrier struct that flows through the [`BlockSource`] and is consumed by
/// projections. Wraps `(Block, BlockOutcome)` so that future carrier metadata
/// (chunk_id, schema version, integrity hash) can be added without touching
/// trait signatures.
///
/// [`BlockSource`]: dango_indexer_historical_block_source::BlockSource
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockData {
    pub block: Block,
    pub outcome: BlockOutcome,
}

impl BlockData {
    /// The block height, taken from `block.info`.
    pub fn height(&self) -> u64 {
        self.block.info.height
    }
}
