//! A benchmark measuring how the chain handles Dango's spot DEX auctions.
//!
//! We arbitrarily choose the blocks 650,001-652,000 in testnet-3. This scripts
//! loads the state right after finalizing the block 650,000, and replays the
//! 2,000 blocks.
//!
//! Make sure to run in `--release` mode:
//!
//! ```bash
//! cargo run -p dango-scripts --example auction_benchmark --release
//! ```

use {
    anyhow::ensure,
    dango_genesis::GenesisCodes,
    grug::{Block, BorshDeExt, Hash256, Query},
    grug_app::{App, Db, NaiveProposalPreparer, NullIndexer},
    grug_db_disk_lite::DiskDbLite,
    grug_vm_rust::RustVm,
    hex_literal::hex,
    std::path::PathBuf,
};

const FROM_HEIGHT: u64 = 650000; // inclusive

const UNTIL_HEIGHT: u64 = 652000; // inclusive

const PRIORITY_MIN: [u8; 24] = hex!("7761736d8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f"); // equals b"wasm" + the dex contract address

const PRIORITY_MAX: [u8; 24] = hex!("7761736d8dd37b7e12d36bbe1c00ce9f0c341bfe1712e740"); // equals b"wasm" + increment_last_byte(the dex contract address)

// Use the following min/max to only load the `dango_dex::state::ORDERS` map:
//
// min = b"wasm" + the dex contract address + len(b"order") as u16 + b"order"
//     = 7761736d8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f00056f72646572
//
// max = equals b"wasm" + the dex contract address + 05 + increment_last_byte(b"order")
//     = 7761736d8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f00056f72646573

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    // Load the DB. If the DB doesn't exist, look for the compressed file and
    // uncompress it.
    let data = cwd.join("data");
    if !data.exists() {
        println!("data folder not found. attempting to uncompress tarball...");

        let result = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(cwd.join(format!("data-{FROM_HEIGHT}.tar.gz")).as_os_str())
            .current_dir(&cwd)
            .output();

        ensure!(
            result.as_ref().is_ok_and(|output| output.status.success()),
            "failed to uncompress tarball: {result:?}"
        );
    }

    // Load the DB. As a basic sanity check, ensure the DB version equals `FROM_HEIGHT`.
    let db = DiskDbLite::open(&data, Some(&(PRIORITY_MIN, PRIORITY_MAX)))?;

    ensure!(
        db.latest_version()
            .is_some_and(|version| version == FROM_HEIGHT),
        "db version doesn't match the from height"
    );

    println!("loaded db");

    // Create the app. As a basic sanity check, ensure the app's last finalized
    // block height equals `FROM_HEIGHT`.
    let _codes = RustVm::genesis_codes();
    let app = App::new(
        db,
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    ensure!(
        app.do_query_app(Query::status(), 0, false)?
            .as_status()
            .last_finalized_block
            .height
            == FROM_HEIGHT,
        "last finalized block height doesn't match the from height"
    );

    println!("created app");

    // Load the blocks.
    let blocks_and_hashes =
        std::fs::read(cwd.join(format!("blocks-{}-{UNTIL_HEIGHT}.borsh", FROM_HEIGHT + 1)))?
            .deserialize_borsh::<Vec<(Block, Hash256)>>()?;

    println!("loaded blocks");

    // Start the timer.
    let start = std::time::Instant::now();

    // Execute the blocks.
    for (block, app_hash) in blocks_and_hashes {
        let block_outcome = app.do_finalize_block(block.clone())?;

        ensure!(
            block_outcome.height == block.info.height,
            "block height mismatch"
        );

        ensure!(
            block_outcome.app_hash == app_hash,
            "app hash mismatch at height {}",
            block.info.height
        );

        app.do_commit()?;
    }

    // Stop the timer.
    let duration = start.elapsed();

    println!("time elapsed: {} seconds", duration.as_secs_f64());

    println!(
        "time elapsed per block: {} ms",
        (duration.as_micros() as f64) / ((UNTIL_HEIGHT - FROM_HEIGHT) as f64) / 1000.
    );

    // Delete the data folder.
    std::fs::remove_dir_all(data)?;

    Ok(())
}
