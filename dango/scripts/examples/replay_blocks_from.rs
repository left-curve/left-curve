use std::path::PathBuf;

use {
    anyhow::ensure,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
};

use {
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_db_memory_lite::MemDbLite,
    grug_vm_rust::RustVm,
};

const FROM_HEIGHT: u64 = 150721; // exclusive

const UNTIL_HEIGHT: u64 = 150722; // inclusive

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let indexer_path = IndexerPath::Dir(cwd.clone());

    let app = App::new(
        MemDbLite::recover(cwd.join(format!("db-{FROM_HEIGHT}.borsh")))?,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    for height in FROM_HEIGHT + 1..=UNTIL_HEIGHT {
        let block_to_index = BlockToIndex::load_from_disk(indexer_path.block_path(height))?;

        let block_outcome = app.do_finalize_block(block_to_index.block)?;

        ensure!(
            block_outcome.app_hash == block_to_index.block_outcome.app_hash,
            "apphash mismatch! height: {height}, expecting: {}, got: {}",
            block_to_index.block_outcome.app_hash,
            block_outcome.app_hash
        );

        app.do_commit()?;
    }

    Ok(())
}
