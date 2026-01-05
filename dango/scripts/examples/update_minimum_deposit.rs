use {
    dango_types::{config::AppConfig, constants::usdc},
    grug::{Addr, Message, QueryClientExt, addr, coins},
    indexer_client::HttpClient,
};

const API_URL: &str = "https://api-mainnet.dango.zone/";

const OWNER_ADDRESS: Addr = addr!("149a2e2bc3ed63aeb0410416b9123d886af1f9cd");

const OWNER_SECRET_PATH: &str = "/Users/larry/.dango/keys/larry.json";

struct MessageBuilder;

#[async_trait::async_trait]
impl dango_scripts::MessageBuilder for MessageBuilder {
    async fn build_message(client: &HttpClient) -> anyhow::Result<Message> {
        let mut app_cfg: AppConfig = client.query_app_config(None).await?;
        app_cfg.minimum_deposit = coins! { usdc::DENOM.clone() => 10_000_000 };

        Ok(Message::configure(None, Some(app_cfg))?)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dango_scripts::send_message::<MessageBuilder>(API_URL, OWNER_SECRET_PATH, OWNER_ADDRESS).await
}
