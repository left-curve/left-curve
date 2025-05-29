use {
    crate::{config::Config, home_directory::HomeDirectory},
    anyhow::anyhow,
    clap::{Parser, Subcommand},
    config_parser::parse_config,
    dango_genesis::GenesisCodes,
    grug_app::{App, AppError, Db, Indexer, NullIndexer},
    grug_db_disk::DiskDb,
    grug_types::{GIT_COMMIT, HashExt},
    grug_vm_hybrid::HybridVm,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath, non_blocking_indexer},
    tokio::task::JoinSet,
};

#[derive(Parser)]
pub struct IndexerCmd {
    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// View a block and results
    Block {
        height: u64,
    },
    /// View a range of blocks and results
    Blocks {
        /// Start height (inclusive)
        start: u64,
        /// End height (inclusive)
        end: u64,
    },
    /// Search for a pattern in the given inclusive range of blocks
    // TODO: make block range optional and figure it out automatically
    Find {
        text: String,
        /// Start height (inclusive)
        start: u64,
        /// End height (inclusive)
        end: u64,
    },
    Reindex,
}

impl IndexerCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        match self.subcmd {
            SubCmd::Block { height } => {
                let indexer_path = IndexerPath::Dir(app_dir.indexer_dir());
                let block_filename = indexer_path.block_path(height);
                let block_to_index = BlockToIndex::load_from_disk(block_filename)?;

                println!("Block: {:#?}", block_to_index.block);
                println!("Block Outcome: {:#?}", block_to_index.block_outcome);
            },
            SubCmd::Blocks { start, end } => {
                let indexer_path = IndexerPath::Dir(app_dir.indexer_dir());
                let mut set = JoinSet::new();

                for block in start..=end {
                    let indexer_path = indexer_path.clone();

                    set.spawn(async move {
                        let block_filename = indexer_path.block_path(block);

                        tokio::task::spawn_blocking(move || {
                            let block_to_index = match BlockToIndex::load_from_disk(block_filename)
                            {
                                Ok(block_to_index) => block_to_index,
                                Err(err) => {
                                    println!("Error loading block {block}: {err}");
                                    return;
                                },
                            };

                            println!("Block: {:#?}", block_to_index.block);
                            println!("Block Outcome: {:#?}", block_to_index.block_outcome);
                        });
                    });
                }

                while let Some(res) = set.join_next().await {
                    if let Err(e) = res {
                        eprintln!("Task panicked: {e}");
                    }
                }
            },
            SubCmd::Find { text, start, end } => {
                let indexer_path = IndexerPath::Dir(app_dir.indexer_dir());
                let mut set = JoinSet::new();

                for block in start..=end {
                    let indexer_path = indexer_path.clone();
                    let text = text.clone();

                    set.spawn(async move {
                        let block_filename = indexer_path.block_path(block);

                        tokio::task::spawn_blocking(move || {
                            let block_to_index = match BlockToIndex::load_from_disk(block_filename)
                            {
                                Ok(block_to_index) => block_to_index,
                                Err(err) => {
                                    eprintln!("Error loading block {block}: {err}");
                                    return;
                                },
                            };

                            let block_text = format!("{:#?}", block_to_index.block);
                            let block_results = format!("{:#?}", block_to_index.block_outcome);

                            if block_text.contains(&text) || block_results.contains(&text) {
                                println!("Found in block {block}:");
                                println!("Block: {:#?}", block_text);
                                println!("Block Outcome: {:#?}", block_results);
                            }
                        });
                    });
                }

                while let Some(res) = set.join_next().await {
                    if let Err(e) = res {
                        eprintln!("Task panicked: {e}");
                    }
                }
            },

            SubCmd::Reindex => {
                tracing::info!("Using git commit: {GIT_COMMIT}");

                // Parse the config file.
                let cfg: Config = parse_config(app_dir.config_file())?;

                // Open disk DB.
                let db = DiskDb::open(app_dir.data_dir())?;

                // // Create Rust VM contract codes.
                // let codes = HybridVm::genesis_codes();

                // // Create hybird VM.
                // let vm = HybridVm::new(cfg.grug.wasm_cache_capacity, [
                //     codes.account_factory.to_bytes().hash256(),
                //     codes.account_margin.to_bytes().hash256(),
                //     codes.account_multi.to_bytes().hash256(),
                //     codes.account_spot.to_bytes().hash256(),
                //     codes.bank.to_bytes().hash256(),
                //     codes.dex.to_bytes().hash256(),
                //     codes.gateway.to_bytes().hash256(),
                //     codes.hyperlane.ism.to_bytes().hash256(),
                //     codes.hyperlane.mailbox.to_bytes().hash256(),
                //     codes.hyperlane.va.to_bytes().hash256(),
                //     codes.lending.to_bytes().hash256(),
                //     codes.oracle.to_bytes().hash256(),
                //     codes.taxman.to_bytes().hash256(),
                //     codes.vesting.to_bytes().hash256(),
                //     codes.warp.to_bytes().hash256(),
                // ]);
                let mut indexer = non_blocking_indexer::IndexerBuilder::default()
                    .with_keep_blocks(true) // ensures block files aren't deleted
                    .with_database_url(&cfg.indexer.database_url)
                    .with_dir(app_dir.indexer_dir())
                    .with_sqlx_pubsub()
                    .with_hooks(dango_indexer_sql::hooks::Hooks)
                    .build()
                    .map_err(|err| anyhow!("failed to build indexer: {err:?}"))?;

                if !indexer.is_database_empty().await? {
                    return Err(anyhow!(
                        "indexer database is not empty, reindexing aborted."
                    ));
                }

                indexer
                    .start(&db.state_storage(None)?)
                    .expect("Can't start indexer");
            },
        }

        Ok(())
    }
}
