use {
    dango_types::{
        constants::{perp_btc, perp_eth, perp_hype, perp_sol},
        oracle::{self, PriceSource},
    },
    grug::{Addr, Coins, Message, addr, btree_map},
    indexer_client::HttpClient,
    pyth_types::constants::{BTC_USD_ID, ETH_USD_ID, HYPE_USD_ID, SOL_USD_ID},
};

const API_URL: &str = "https://api-testnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("c4a8f7bbadd1457092a8cd182480230c0a848331");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/testnet-owner.json";

const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        Ok(Message::execute(
            ORACLE,
            &oracle::ExecuteMsg::RegisterPriceSources(btree_map! {
                perp_btc::DENOM.clone() => PriceSource::Pyth {
                    id: BTC_USD_ID.id,
                    channel: BTC_USD_ID.channel,
                    precision: 8,
                },
                perp_eth::DENOM.clone() => PriceSource::Pyth {
                    id: ETH_USD_ID.id,
                    channel: ETH_USD_ID.channel,
                    precision: 18,
                },
                perp_sol::DENOM.clone() => PriceSource::Pyth {
                    id: SOL_USD_ID.id,
                    channel: SOL_USD_ID.channel,
                    precision: 9,
                },
                perp_hype::DENOM.clone() => PriceSource::Pyth {
                    id: HYPE_USD_ID.id,
                    channel: HYPE_USD_ID.channel,
                    precision: 8, // HyperCore decimal places
                },
            }),
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
