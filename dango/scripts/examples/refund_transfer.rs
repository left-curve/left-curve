use {
    dango_types::{
        constants::eth,
        gateway::{self, Remote},
    },
    grug::{Addr, Message, QueryClientExt, addr, coins},
    hyperlane_types::{addr32, constants::ethereum},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const GATEWAY: Addr = addr!("c51e2cbe9636a90c86463ac3eb18fbee92b700d1");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(client: &HttpClient) -> anyhow::Result<Message> {
        // Query and refund the owner account's entire ETH balance.
        let balance = client
            .query_balance(OWNER_ADDRESS, eth::DENOM.clone(), None)
            .await?;

        Ok(Message::execute(
            GATEWAY,
            &gateway::ExecuteMsg::TransferRemote {
                remote: Remote::Warp {
                    domain: ethereum::DOMAIN,
                    contract: ethereum::ETH_WARP,
                },
                recipient: addr32!(
                    "00000000000000000000000016eca0f72ca0a91a5989397b3c405dd8c23dfa52"
                ),
            },
            coins! { eth::DENOM.clone() => balance },
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
