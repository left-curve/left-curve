use {
    anyhow::{anyhow, ensure},
    dango_genesis::GenesisCodes,
    grug::{
        BlockInfo, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT, GenesisState, Json, JsonDeExt,
        Timestamp,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer},
    grug_db_memory_lite::MemDbLite,
    grug_vm_rust::RustVm,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
    std::{fs, path::PathBuf},
};

const GENESIS_TIMESTAMP: Timestamp = Timestamp::from_seconds(31536000); // 1971-01-01T00:00:00Z

const CHAIN_ID: &str = "dev-6";

const UNTIL_HEIGHT: u64 = 150721; // inclusive

fn main() -> anyhow::Result<()> {
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");

    let indexer_path = IndexerPath::Dir(cwd.clone());

    let cometbft_genesis = fs::read(cwd.join("genesis.json"))?.deserialize_json::<Json>()?;

    let genesis_state = cometbft_genesis
        .as_object()
        .ok_or_else(|| anyhow!("cometbft genesis file isn't a json map"))?
        .get("app_state")
        .ok_or_else(|| anyhow!("cometbft genesis file doesn't have a `app_state` field"))?
        .clone()
        .deserialize_json::<GenesisState>()?;

    let _codes = RustVm::genesis_codes();

    let app = App::new(
        MemDbLite::new(),
        RustVm::new(),
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
    );

    let _app_hash = app.do_init_chain(
        CHAIN_ID.to_string(),
        BlockInfo {
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: GENESIS_TIMESTAMP,
            hash: GENESIS_BLOCK_HASH,
        },
        genesis_state,
    )?;

    for height in 1..=UNTIL_HEIGHT {
        let block_to_index = BlockToIndex::load_from_disk(indexer_path.block_path(height))?;

        let block_outcome = app.do_finalize_block(block_to_index.block)?;

        ensure!(
            block_outcome.app_hash == block_to_index.block_outcome.app_hash,
            "apphash mismatch! height: {height}, expecting: {}, got: {}",
            block_to_index.block_outcome.app_hash,
            block_outcome.app_hash
        );

        app.do_commit()?;

        if height % 1000 == 0 {
            println!("processed block {height}");
        }
    }

    app.db.dump(cwd.join(format!("db-{UNTIL_HEIGHT}.borsh")))
}
