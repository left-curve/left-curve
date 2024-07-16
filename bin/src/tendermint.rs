use {crate::prompt::print_json_pretty, clap::Parser, grug_sdk::Client};

#[derive(Parser)]
pub struct StatusCmd {
    /// Tendermint RPC address
    #[arg(long, default_value = "http://127.0.0.1:26657")]
    node: String,
}

impl StatusCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let client = Client::connect(&self.node)?;
        print_json_pretty(client.query_status().await?)
    }
}
