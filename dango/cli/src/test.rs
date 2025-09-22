use {
    crate::{config::Config, home_directory::HomeDirectory},
    clap::{Parser, Subcommand},
    config_parser::parse_config,
    grug::Inner,
    grug_types::NonEmpty,
    pyth_lazer::PythClientLazerCache,
    pyth_types::{
        PayloadData, PythClientTrait,
        constants::{BTC_USD_ID_LAZER, ETH_USD_ID_LAZER},
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
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        match self.subcmd {
            SubCmd::Pyth => {
                // Parse the config file.
                let cfg: Config = parse_config(app_dir.config_file())?;

                // For the purpose of this test, we fetch the prices of BTC and ETH.
                let ids = NonEmpty::new_unchecked(vec![BTC_USD_ID_LAZER, ETH_USD_ID_LAZER]);

                let mut client = PythClientLazerCache::new(
                    NonEmpty::new(cfg.pyth.endpoints)?,
                    cfg.pyth.access_token,
                )?;

                let mut stream = client.stream(ids).await?;

                loop {
                    let Some(data) = stream.next().await else {
                        continue;
                    };

                    // Decode the price feeds.
                    let mut feeds = Vec::with_capacity(2);
                    for message in data.into_inner() {
                        // For the purpose of this test, it isn't necessary to verify the Wormhole VAAs.

                        // Deserialize the payload.
                        let payload = PayloadData::deserialize_slice_le(&message.payload)?;
                        feeds.push(payload);
                    }

                    info!("Fetched data:\n{feeds:?}");
                }
            },
        }
    }
}
