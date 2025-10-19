use {
    anyhow::ensure,
    dango_genesis::GenesisCodes,
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_db_disk_lite::DiskDbLite,
    grug_vm_rust::RustVm,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
    std::path::PathBuf,
};

const FROM_HEIGHT: u64 = 639636;

const UNTIL_HEIGHT: u64 = 650000; // inclusive

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let indexer_path = IndexerPath::Dir(cwd.clone());

    let db = DiskDbLite::open(cwd.join("data"), None)?;

    let _codes = RustVm::genesis_codes();

    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    for height in (FROM_HEIGHT + 1)..=UNTIL_HEIGHT {
        let block_to_index = BlockToIndex::load_from_disk(indexer_path.block_path(height))?;

        let block_outcome = app.do_finalize_block(block_to_index.block)?;

        ensure!(
            block_outcome.app_hash == block_to_index.block_outcome.app_hash,
            "apphash mismatch! height: {height}, expecting: {}, got: {}",
            block_to_index.block_outcome.app_hash,
            block_outcome.app_hash
        );

        app.do_commit()?;

        println!("commit block {}", block_outcome.height);
    }

    println!("done!");

    Ok(())
}
