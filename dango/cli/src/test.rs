use {
    clap::{Parser, Subcommand},
    grug_types::{JsonSerExt, MockApi, NonEmpty},
    pyth_client::{PythClient, PythClientTrait},
    pyth_types::{
        PythVaa,
        constants::{BTC_USD_ID, ETH_USD_ID, PYTH_URL},
    },
    tokio_stream::StreamExt,
    tracing::info,
};

#[derive(Parser)]
pub struct TestCmd {
    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Fetch and print price feeds from Pyth stream API
    Pyth,
}

impl TestCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.subcmd {
            SubCmd::Pyth => {
                // For the purpose of this test, we fetch the prices of BTC and ETH.
                let ids = NonEmpty::new_unchecked(vec![BTC_USD_ID, ETH_USD_ID]);

                let mut client = PythClient::new(PYTH_URL)?;
                let mut stream = client.stream(ids).await?;

                loop {
                    let Some(data) = stream.next().await else {
                        continue;
                    };

                    // Decode the price feeds.
                    let mut feeds = Vec::with_capacity(2);
                    for raw in data {
                        let vaa = PythVaa::new(&MockApi, raw.as_ref())?;
                        // For the purpose of this test, it isn't necessary to verify the Wormhole VAAs.
                        feeds.extend(vaa.unverified());
                    }

                    info!("Fetched data\n{}", feeds.to_json_string_pretty()?);
                }
            },
        }
    }
}
