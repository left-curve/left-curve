use {
    dango_types::{
        constants::{eth, usdc},
        gateway::{self, Origin, Remote},
    },
    grug::{Addr, Coins, Message, addr, btree_set},
    hyperlane_types::{addr32, constants::ethereum},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const GATEWAY: Addr = addr!("c51e2cbe9636a90c86463ac3eb18fbee92b700d1");

const WARP: Addr = addr!("981e6817442143ce5128992c7ab4a317321f00e9");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        Ok(Message::execute(
            GATEWAY,
            &gateway::ExecuteMsg::SetRoutes(btree_set! {
                (
                    Origin::Remote(eth::SUBDENOM.clone()),
                    WARP,
                    Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: addr32!("0000000000000000000000009d259aa1ec7324c7433b89d2935b08c30f3154cb"),
                    },
                ),
                (
                    Origin::Remote(usdc::SUBDENOM.clone()),
                    WARP,
                    Remote::Warp {
                        domain: ethereum::DOMAIN,
                        contract: addr32!("000000000000000000000000d05909852ae07118857f9d071781671d12c0f36c"),
                    },
                ),
            }),
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
