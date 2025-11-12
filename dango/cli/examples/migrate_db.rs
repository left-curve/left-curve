use {
    dango_genesis::GenesisCodes,
    grug_app::{App, Db, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_disk::DiskDb,
    grug_types::{JsonSerExt, Query, addr, json},
    grug_vm_rust::RustVm,
    std::path::PathBuf,
};

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join("data");

    // -------------------------- Do the DB migration --------------------------

    println!("running migration...");

    dango_cli::db_migration::migrate_db(&path)?;

    println!("done!");

    // ------------------ Some checks to make sure it worked -------------------

    let db = DiskDb::<SimpleCommitment>::open(path)?;

    println!("latest version: {:?}", db.latest_version());
    println!("oldest version: {:?}", db.oldest_version());
    println!("root hash at latest version: {:?}", db.root_hash(None));

    let _codes = RustVm::genesis_codes();

    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
        "",
    );

    println!(
        "status: {}",
        app.do_query_app(Query::status(), None, false)?
            .as_status()
            .to_json_string_pretty()?
    );

    println!(
        "orders in BTC-USD pool: {}",
        app.do_query_app(
            Query::wasm_smart(
                addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f"),
                &json!({
                    "orders_by_pair": {
                        "base_denom": "bridge/btc",
                        "quote_denom": "bridge/usdc"
                    }
                })
            )?,
            None,
            false,
        )?
        .as_wasm_smart()
        .to_json_string_pretty()?
    );

    Ok(())
}
