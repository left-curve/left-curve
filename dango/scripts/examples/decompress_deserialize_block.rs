//! Deserialize a compressed, encoded block file dumped by the indexer disk saver.
//! Useful for debugging a chain crash.

use {
    grug::{BorshDeExt, JsonSerExt},
    indexer_sql::block_to_index::BlockToIndex,
};

const BLOCK: &[u8] = include_bytes!("./39847.borsh.xz");

fn main() -> anyhow::Result<()> {
    // Decompress.
    let compressed = BLOCK.to_vec();
    let mut decompressed = Vec::new();
    lzma_rs::lzma_decompress(&mut compressed.as_slice(), &mut decompressed)?;

    // Deserialize.
    let decoded = decompressed.deserialize_borsh::<BlockToIndex>()?;

    // Save it to file as JSON.
    std::fs::write("./39847.json", decoded.to_json_string_pretty()?)?;

    Ok(())
}
