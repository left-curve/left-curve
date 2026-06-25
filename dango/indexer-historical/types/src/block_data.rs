use {
    borsh::{BorshDeserialize, BorshSerialize},
    dango_primitives::{Block, BlockOutcome},
    serde::{Deserialize, Serialize},
};

/// Carrier struct that flows through the [`BlockSource`] and is consumed by
/// projections. Wraps `(Block, BlockOutcome)` so that future carrier metadata
/// (chunk_id, schema version, integrity hash) can be added without touching
/// trait signatures.
///
/// Derives borsh so the detached `RemoteBlockSource` can persist it to its raw
/// store (the on-disk format the dango node already uses for block cache files),
/// and serde so it decodes **directly** from the node's `/block/full/*` REST and
/// `full_block` subscription JSON — where the outcome field is named
/// `block_outcome` (the same name the node's cache files use), mapped below.
///
/// [`BlockSource`]: dango_indexer_historical_block_source::BlockSource
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BlockData {
    pub block: Block,
    #[serde(rename = "block_outcome")]
    pub outcome: BlockOutcome,
}

impl BlockData {
    /// The block height, taken from `block.info`.
    pub fn height(&self) -> u64 {
        self.block.info.height
    }
}
