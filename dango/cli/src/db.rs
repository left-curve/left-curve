use {crate::home_directory::HomeDirectory, clap::Subcommand, colored::Colorize, std::fs};

#[derive(Subcommand)]
pub enum DbCmd {
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
