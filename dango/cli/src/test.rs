use {
    clap::{Parser, Subcommand},
    pyth_client::cli_pyth_stream,
};

#[derive(Parser)]
pub struct TestCmd {
    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Run Pyth stream
    Pyth {},
}

impl TestCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.subcmd {
            SubCmd::Pyth {} => {
                cli_pyth_stream().await;
                Ok(())
            },
        }
    }
}
