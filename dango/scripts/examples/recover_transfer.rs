use {
    dango_types::bank,
    grug::{Addr, Coins, Message, addr},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

const BANK: Addr = addr!("e0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7");

const GATEWAY: Addr = addr!("c51e2cbe9636a90c86463ac3eb18fbee92b700d1");

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(_client: &HttpClient) -> anyhow::Result<Message> {
        Ok(Message::execute(
            BANK,
            &bank::ExecuteMsg::RecoverTransfer {
                sender: GATEWAY,
                recipient: addr!("9b16ff5f6e92654e6186ae331fb7d2091d66e17b"),
            },
            Coins::new(),
        )?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
