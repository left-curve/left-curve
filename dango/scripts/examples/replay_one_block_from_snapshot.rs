use {
    anyhow::ensure,
    dango_genesis::GenesisCodes,
    grug::JsonSerExt,
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_db_memory_lite::MemDbLite,
    grug_vm_rust::RustVm,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
    std::path::PathBuf,
};

const SNAPSHOT: &str = "db-150721.borsh";

const HEIGHT: u64 = 150722;

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let indexer_path = IndexerPath::Dir(cwd.clone());

    let _codes = RustVm::genesis_codes();

    let app = App::new(
        MemDbLite::recover(cwd.join(SNAPSHOT))?,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    let block_to_index = BlockToIndex::load_from_disk(indexer_path.block_path(HEIGHT))?;

    let block_outcome = app.do_finalize_block(block_to_index.block)?;

    ensure!(
        block_outcome.app_hash == block_to_index.block_outcome.app_hash,
        "apphash mismatch! height: {HEIGHT}, expecting: {}, got: {}",
        block_to_index.block_outcome.app_hash,
        block_outcome.app_hash
    );

    println!("block_outcome: {}", block_outcome.to_json_string_pretty()?);

    app.do_commit()?;

    Ok(())
}
