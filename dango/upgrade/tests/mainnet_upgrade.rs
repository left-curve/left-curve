use {
    dango_genesis::GenesisCodes,
    grug::{Buffer, Shared},
    grug_app::{Db, LAST_FINALIZED_BLOCK, SimpleCommitment},
    grug_db_disk::DiskDb,
    grug_vm_rust::RustVm,
    std::path::PathBuf,
};

#[test]
fn mainnet_upgrade_succeeds() {
    let db_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("deploy/downloaded-db/mainnet/inter1/data");

    if !db_path.exists() {
        eprintln!("Skipping test: mainnet DB not found at {}", db_path.display());
        return;
    }

    // Register contract wrappers so the RustVm can resolve code hashes.
    let _codes = RustVm::genesis_codes();

    let db = DiskDb::<SimpleCommitment>::open(&db_path).unwrap();
    let storage = db.state_storage(None).unwrap();

    let block = LAST_FINALIZED_BLOCK.load(&storage).unwrap();

    // Wrap in a writable buffer, matching how app.rs calls the upgrade handler.
    let buffer = Shared::new(Buffer::new(storage, None, "upgrade_test"));

    // Run the upgrade handler. The invariant assertions inside will panic on
    // failure, so reaching the end means everything passed.
    dango_upgrade::do_upgrade(Box::new(buffer), RustVm::new(), block).unwrap();
}
