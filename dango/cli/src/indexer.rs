use {
    crate::{config::Config, home_directory::HomeDirectory},
    clap::{Parser, Subcommand},
    config_parser::parse_config,
    indexer_sql::{block_to_index::BlockToIndex, indexer_path::IndexerPath},
    metrics_exporter_prometheus::PrometheusHandle,
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
    MetricsHttpd,
}

impl IndexerCmd {
    pub async fn run(
        self,
        app_dir: HomeDirectory,
        metrics_handler: PrometheusHandle,
    ) -> anyhow::Result<()> {
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

            SubCmd::MetricsHttpd => {
                let cfg: Config = parse_config(app_dir.config_file())?;

                tracing::info!(
                    "Starting metrics HTTP server at {}:{}",
                    &cfg.indexer.httpd.ip,
                    cfg.indexer.httpd.metrics_port
                );

                // Run the metrics HTTP server
                indexer_httpd::server::run_metrics_server(
                    &cfg.indexer.httpd.ip,
                    cfg.indexer.httpd.metrics_port,
                    metrics_handler,
                )
                .await?;
            },
        }

        Ok(())
    }
}
