//! This scripts loads blocks in a given range of block height from the indexer
//! folder, strip off unnecessary info, and compress them into a single file.
//! This is to be used in the benchmark for auction performance.

use {
    grug::BorshSerExt,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
    std::path::PathBuf,
};

const FROM_HEIGHT: u64 = 650001; // inclusive

const UNTIL_HEIGHT: u64 = 652000; // inclusive

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let indexer_path = IndexerPath::Dir(cwd.clone());

    let blocks = (FROM_HEIGHT..=UNTIL_HEIGHT)
        .map(|height| {
            let block_to_index = BlockToIndex::load_from_disk(indexer_path.block_path(height))?;
            Ok((block_to_index.block, block_to_index.block_outcome.app_hash))
        })
        .collect::<indexer_sql::Result<Vec<_>>>()?;

    std::fs::write(
        cwd.join(format!("blocks-{FROM_HEIGHT}-{UNTIL_HEIGHT}.borsh")),
        blocks.to_borsh_vec()?,
    )?;

    Ok(())
}
