use {
    crate::prompt::confirm,
    clap::Parser,
    colored::Colorize,
    std::{fs, path::PathBuf},
};

#[derive(Parser)]
pub struct ResetCmd {
    /// Skip confirmation
    #[arg(short, long)]
    yes: bool,
}

impl ResetCmd {
    pub fn run(self, data_dir: PathBuf) -> anyhow::Result<()> {
        if !data_dir.exists() {
            println!("Data directory {data_dir:?} not found, nothing to do.");
            return Ok(());
        }

        if !self.yes {
            confirm(format!("Confirm deleting data directory {data_dir:?}?").bold())?;
        }

        Ok(fs::remove_dir_all(data_dir)?)
    }
}
