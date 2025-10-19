//! Load a `DiskDbLite`, and ensure the DB version and last finalized block
//! height match. This for quickly detecting whether the DB is corrupted.

use {
    anyhow::{anyhow, ensure},
    dango_genesis::GenesisCodes,
    grug::Query,
    grug_app::{App, Db, NaiveProposalPreparer, NullIndexer},
    grug_db_disk_lite::DiskDbLite,
    grug_vm_rust::RustVm,
    std::path::PathBuf,
};

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let db = DiskDbLite::open(cwd.join("data"))?;
    let db_version = db.latest_version().ok_or(anyhow!("db version is `None`"))?;

    println!("db version: {db_version}");

    let _codes = RustVm::genesis_codes();

    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    let status = app
        .do_query_app(Query::status(), db_version, false)?
        .as_status();

    println!(
        "last finalized block height: {}",
        status.last_finalized_block.height
    );

    ensure!(
        db_version == status.last_finalized_block.height,
        "db version doesn't match last finalized block height!"
    );

    Ok(())
}
