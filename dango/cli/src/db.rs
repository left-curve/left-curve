use {
    crate::home_directory::HomeDirectory,
    clap::Subcommand,
    colored::Colorize,
    grug_app::{Db, SimpleCommitment},
    grug_db_disk::DiskDb,
    std::fs,
};

#[derive(Subcommand)]
pub enum DbCmd {
    /// Print the database version
    Version,
    /// Prune the database
    Prune {
        /// Delete historical states up to this height (exclusive)
        up_to_version: u64,

        /// Force an immediate database compaction following the pruning
        #[arg(long)]
        compact: bool,
    },
    /// Delete the entire database
    Reset {
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

impl DbCmd {
    pub fn run(self, dir: HomeDirectory) -> anyhow::Result<()> {
        let data_dir = dir.data_dir();

        if !data_dir.exists() {
            println!("Data directory {data_dir:?} not found, nothing to do.");
            return Ok(());
        }

        match self {
            DbCmd::Version => {
                let db = DiskDb::<SimpleCommitment>::open(dir.data_dir())?;

                println!("Latest version: {:?}", db.latest_version());
                println!("Oldest version: {:?}", db.oldest_version());
            },
            DbCmd::Prune {
                up_to_version,
                compact,
            } => {
                let db = DiskDb::<SimpleCommitment>::open(dir.data_dir())?;

                db.prune(up_to_version)?;

                if compact {
                    db.compact();
                }
            },
            DbCmd::Reset { yes } => {
                if !yes {
                    confirm(
                        format!(
                            "Confirm deleting data directory {data_dir:?}? This operation is irreversible."
                        )
                        .bold()
                        .to_string(),
                    )?;
                }

                fs::remove_dir_all(data_dir)?;
            },
        }

        Ok(())
    }
}

fn confirm<T>(prompt: T) -> dialoguer::Result<bool>
where
    T: Into<String>,
{
    dialoguer::Confirm::new().with_prompt(prompt).interact()
}
