use {
    clap::Subcommand,
    colored::Colorize,
    grug_app::PrunableDb,
    grug_db_disk::DiskDb,
    std::{fs, path::PathBuf},
};

#[derive(Subcommand)]
pub enum DbCmd {
    /// Delete data up to a version
    Prune {
        /// Cutoff version for the pruning
        up_to_version: u64,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Delete the entire database
    Reset {
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

impl DbCmd {
    pub fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        if !data_dir.exists() {
            println!("Data directory {data_dir:?} not found, nothing to do.");
            return Ok(());
        }

        match self {
            DbCmd::Prune { up_to_version, yes } => {
                if !yes {
                    confirm(
                        format!(
                            "Confirm pruning data up to version {up_to_version}? This operation is irreversible."
                        )
                        .bold()
                        .to_string(),
                    )?;
                }

                Ok(DiskDb::open(data_dir)?.prune(up_to_version)?)
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

                Ok(fs::remove_dir_all(data_dir)?)
            },
        }
    }
}

fn confirm<T>(prompt: T) -> dialoguer::Result<bool>
where
    T: Into<String>,
{
    dialoguer::Confirm::new().with_prompt(prompt).interact()
}
