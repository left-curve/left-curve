use {crate::home_directory::HomeDirectory, anyhow::ensure, clap::Parser};

#[derive(Parser)]
pub struct InitCmd;

impl InitCmd {
    pub fn run(&self, home: &HomeDirectory) -> anyhow::Result<()> {
        ensure!(
            !home.exists(),
            "home directory already exists: {}",
            home.as_os_str().to_str().unwrap()
        );

        std::fs::create_dir_all(home)?;
        std::fs::create_dir(home.data_dir())?;
        std::fs::create_dir(home.keys_dir())?;
        std::fs::create_dir(home.indexer_dir())?;

        std::fs::write(
            home.config_file(),
            include_str!("../testdata/default_config.toml"),
        )?;

        tracing::info!(
            "Dango directory initiated at: {}",
            home.as_os_str().to_str().unwrap()
        );

        Ok(())
    }
}
